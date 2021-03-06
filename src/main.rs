#![allow(non_snake_case)]

extern crate rustc_serialize;
extern crate rusqlite;
extern crate threadpool;
extern crate regex;

use std::sync::mpsc::channel;
use std::sync::mpsc::{Sender,Receiver};
use std::thread;
use std::io::prelude::*;
use std::net::{TcpListener, TcpStream};
use threadpool::ThreadPool;
use regex::Regex;

#[derive(Debug)]
struct Command {
    dir: String,
    command: String,
    file: String,
}

enum CommandIndexJob {
    Stop,
    ProcessCompCommands{ path: String, context: String },
    IndexSource{ comm: Command, context: String },
}

enum SqliteJob {
    Stop,
    RunQuery(String),
    InsertMany{ files: Vec<String>,context: String,dir: String,flags: String },
    QueryFile{ context: String, path: String, txCmd: Sender<
        Result< Command, String > > },
}

struct MyAppInstance {
    indexSender: Sender<CommandIndexJob>,
    sqliteQuerySender: Sender< SqliteJob >,
    queryResultSender: Sender< Result< Command, String > >,
    queryResultReceiver: Receiver< Result< Command, String > >,
    endRecv: Receiver<i32>,
}

fn handleStream(mut inst: &MyAppInstance, mut stream: TcpStream) -> bool {
    println!("DOIN IT");
    let mut theStr = String::with_capacity(1024 * 2);
    stream.read_to_string(&mut theStr);
    println!("{}",theStr);
    let spl: Vec<String> = theStr.split("|")
        .map(|slice| { String::from(slice) }).collect();
    let firstTrimmed = String::from(spl[0].trim());
    if firstTrimmed == "p" { // process
        let context = String::from(spl[1].trim());
        if context != "" {
            let jerb = CommandIndexJob::ProcessCompCommands {
                path: String::from(spl[2].trim()),
                context: context,
            };
            inst.indexSender.send(jerb);
        } else {
            println!("Context must not be empty!");
        }
    } else if firstTrimmed == "s" { // stop
        println!("End signal received, shutting down...");
        inst.indexSender.send(CommandIndexJob::Stop);
        inst.sqliteQuerySender.send(SqliteJob::Stop);
        inst.endRecv.recv();
        inst.endRecv.recv();
        return false;
    } else if firstTrimmed == "q" {
        println!("Query about to be served...");
        let context = String::from(spl[1].trim());
        let path = String::from(spl[2].trim());
        inst.sqliteQuerySender.send(
            SqliteJob::QueryFile{
                context: context.clone(),
                path: path.clone(),
                txCmd: inst.queryResultSender.clone(),
            }
        );
        let out = inst.queryResultReceiver.recv().unwrap();
        let resp =
            match out {
                Ok(res) => {
                    format!("g|{}|{}|{}\r\n",context,path,res.command)
                },
                Err(err) => {
                    format!("e|cannot query for flags\r\n")
                }
            };
        println!("Query served [{}]",resp);
        stream.write_all(&resp.into_bytes());
        stream.shutdown(std::net::Shutdown::Both);
    }

    println!("END CONNECTION");
    return true;
}

fn parseCommands(string: &String) -> Vec<Command> {
    use rustc_serialize::json::Json;
    let data = Json::from_str(&string).unwrap();

    let mut commands = Vec::with_capacity(16);
    let arr = data.as_array().unwrap();
    for i in arr {
        let obj = i.as_object().unwrap();
        let comm = Command {
            dir: obj.get("directory").unwrap()
                .as_string().unwrap().to_string(),
            command: obj.get("command").unwrap()
                .as_string().unwrap().to_string(),
            file: obj.get("file").unwrap()
                .as_string().unwrap().to_string(),
        };
        commands.push(comm);
    }
    commands
}

fn hitTheFile(filepath: String,projectName: String,send: Sender<CommandIndexJob>) {
    use std::io::prelude::*;
    use std::fs::File;

    println!("About to index |{}|",filepath);
    let mut f = File::open(&filepath);
    match f {
        Ok(mut succ) => {
            let mut contents = String::with_capacity(1024 * 64);
            succ.read_to_string(&mut contents);
            let parseRes = parseCommands(&contents);
            for i in parseRes {
                send.send(CommandIndexJob::IndexSource{
                    comm: i, context: projectName.clone() });
            }
        },
        Err(err) => {
            println!("File open error: {}",err);
        }
    }
}

fn argArray(target: &str) -> Vec<String> {
    let mut res = Vec::<String>::with_capacity(3);
    res.push("/bin/sh".to_string());
    res.push("-c".to_string());
    res.push(target.to_string());
    res
}

#[test]
fn test_arg_splitter() {
    let theStr = "g++   shazzlow\\ cxx \"stuff wit space\" -g -o yo.txt";
    let out = argArray(theStr);
    assert!( out[0] == "/bin/sh" );
    assert!( out[1] == "-c" );
    assert!( out[2] == theStr );
}

#[test]
fn test_out_parser() {
    let theStr = "ChainFunctor.o: \\\n
/home/deividas/Desktop/ramdisk/dir\\ wit\\ space/tests-templatious/ChainFunctor.cpp \\\n
/usr/include/stdc-predef.h /usr/include/c++/4.9/cstring \\\n
/home/deividas/Desktop/ramdisk/dir\\ wit\\ space/tests-templatious/detail/ConstructorCountCollection.hpp \\\n
/home/deividas/Desktop/ramdisk/dir\\ wit\\ space/tests-templatious/detail/../TestDefs.hpp\\\n";
    let conv = theStr.to_string();

    let out = parseFileList(&conv);
    assert!( out[0] == "/home/deividas/Desktop/ramdisk/dir wit space/tests-templatious/ChainFunctor.cpp" );
    assert!( out[1] == "/usr/include/stdc-predef.h" );
    assert!( out[2] == "/usr/include/c++/4.9/cstring" );
    assert!( out[3] == "/home/deividas/Desktop/ramdisk/dir wit space/tests-templatious/detail/ConstructorCountCollection.hpp" );
    assert!( out[4] == "/home/deividas/Desktop/ramdisk/dir wit space/tests-templatious/TestDefs.hpp" );
}

fn parseFileList(theString: &String) -> Vec<String> {
    let rplStr = theString.replace("\\ ","@@@");
    let rgx = Regex::new(r"([@/\w\._+:-]+)").unwrap();
    let mut res = Vec::with_capacity(64);

    for i in rgx.captures_iter(&rplStr) {
        let grCap = i.at(1).unwrap().to_string();
        let replBack = grCap.replace("@@@","\\ ");
        if (!replBack.contains(":")) {
            let resolved = resolveToAbsPath(&replBack);
            res.push(resolved);
        }
    }

    res
}

fn resolveToAbsPath(relPath: &String) -> String {
    let repl = relPath.replace("\\","");
    let trimRgx = Regex::new(r"^\s*(.*?)\s*$").unwrap();
    let slashRepRgx = Regex::new(r"/[^/]+/\.\./").unwrap();
    let trimmed = trimRgx.replace_all(&repl,"$1");
    let doubleDotRemoved = slashRepRgx.replace_all(&trimmed,"/");
    return doubleDotRemoved;
}

fn indexSource(comm: Command,context: String,send: Sender<SqliteJob>) {
    let dropOut = Regex::new(r"^(.*?)[\s]+-o[\s]+.*?\s(-.*?)$").unwrap();
    let replCmd = dropOut.replace_all(&comm.command,"$1 -M $2");
    let arr = argArray(&replCmd);

    let mut procVar = std::process::Command::new(&arr[0]);
    procVar.current_dir(&comm.dir);

    let argIter = arr.iter().skip(1);
    for i in argIter {
        procVar.arg(&*i);
    }

    let output = procVar.output().unwrap();
    let headerString = String::from_utf8(output.stdout).unwrap();

    let mut fileList = parseFileList(&headerString);
    fileList.push(comm.file);

    let jerb = SqliteJob::InsertMany{
        files: fileList,
        context: context,
        dir: comm.dir,
        flags: comm.command,
    };

    send.send(jerb);

    let filterOutDupes = SqliteJob::RunQuery(
        "DELETE FROM flags
         WHERE id NOT IN (
             SELECT MAX(id)
             FROM flags
             GROUP BY filename, context
         )".to_string()
    );
    send.send(filterOutDupes);
}

fn listen(inst: MyAppInstance) {
    let listener = TcpListener::bind("127.0.0.1:7777");

    match listener {
        Ok(listen) => {
            for stream in listen.incoming() {
                match stream {
                    Ok(stream) => {
                        let res = handleStream(&inst,stream);
                        if !res { return; };
                    }
                    Err(e) => {
                        println!("{}",e);
                    }
                }
            }
        },
        Err(err) => {
            println!("Coudln't open stream: {}",err);
        },
    }
}

fn main() {
    use rusqlite::SqliteConnection;

    let (txJob,rxJob) = channel::<CommandIndexJob>();
    let (txQuery,rxQuery) = channel::<SqliteJob>();
    let (txQueryFile,rxQueryFile) = channel();
    let (txEnd,rxEnd) = channel::<i32>();
    let clonedTxJob = txJob.clone();
    let clonedTxQuery = txQuery.clone();
    let inst = MyAppInstance {
        indexSender: txJob,
        sqliteQuerySender: txQuery,
        queryResultSender: txQueryFile,
        queryResultReceiver: rxQueryFile,
        endRecv: rxEnd,
    };

    let dbConn = SqliteConnection::open(&"flags.sqlite").unwrap();
    dbConn.execute("CREATE TABLE IF NOT EXISTS flags(
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        context  TEXT,
        filename TEXT,
        dir      TEXT,
        flags    TEXT
    );",&[]);
    let dbEndClone = txEnd.clone();
    thread::spawn(move|| {
        let mut keepGoing = true;
        let mut stmt = dbConn.prepare("
           SELECT filename,dir,flags FROM flags
           WHERE context==$1 AND filename==$2;
        ").unwrap();
        while (keepGoing) {
            let res = rxQuery.recv().unwrap();
            match res {
                SqliteJob::Stop => {
                    keepGoing = false;
                },
                SqliteJob::RunQuery(msg) => {
                    println!("Running filter query...");
                    dbConn.execute(&msg,&[]);
                    println!("Ran!");
                },
                SqliteJob::InsertMany{ files: vec, context: ctx, dir: dir, flags: flg } => {
                    println!("Inserting {} files...",vec.len());
                    dbConn.execute("BEGIN;",&[]);
                    for i in vec {
                        dbConn.execute("
                            INSERT INTO flags (
                                context,
                                filename,
                                dir,
                                flags
                            ) VALUES ($1,$2,$3,$4);
                        ",&[&ctx,&i,&dir,&flg]);
                    }
                    dbConn.execute("COMMIT;",&[]);
                    println!("Inserted!");
                },
                SqliteJob::QueryFile{ context: ctx, path: path, txCmd: txCmd } => {
                    let iter = stmt.query_map(&[&ctx,&path], |row| {
                        Command {
                            file: row.get(0),
                            dir: row.get(1),
                            command: row.get(2),
                        }
                    });

                    match iter {
                        Ok(mut theIter) => {
                            let next = theIter.next();
                            match next {
                                Some(presend) => {
                                    match presend {
                                        Ok(toSend) => txCmd.send(Ok(toSend)),
                                        Err(err) => txCmd.send(Err(format!("{}",err))),
                                    }
                                },
                                None => {
                                    txCmd.send(Err(format!("no-flags-found")))
                                },
                            };
                        },
                        Err(err) => {
                            println!("Sqlite error: {}",err);
                            txCmd.send(Err(format!("Sqlite error: {}",err)));
                        },
                    };
                },
            }
        }
        dbEndClone.send(7);
    });

    let idxEndClone = txEnd.clone();
    thread::spawn(move|| {
        let pool = ThreadPool::new(8);
        let mut keepGoing = true;
        while (keepGoing) {
            let res = rxJob.recv().unwrap();
            match res {
                CommandIndexJob::Stop => {
                    keepGoing = false;
                },
                CommandIndexJob::ProcessCompCommands{ path: p, context: ctx } => {
                    let clonedTxJob = clonedTxJob.clone();
                    pool.execute(move|| {
                        hitTheFile(p,ctx,clonedTxJob);
                    });
                },
                CommandIndexJob::IndexSource{ comm: cmd, context: ctx } => {
                    let clonedTxQuery = clonedTxQuery.clone();
                    pool.execute(move|| {
                        indexSource(cmd,ctx,clonedTxQuery);
                    });
                },
            };
        }
        idxEndClone.send(7);
    });

    listen(inst);
}
