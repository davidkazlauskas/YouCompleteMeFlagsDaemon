#![allow(non_snake_case)]

extern crate rustc_serialize;
extern crate rusqlite;
extern crate threadpool;
extern crate regex;

use std::sync::mpsc::channel;
use std::sync::mpsc::Sender;
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
}

struct MyAppInstance {
    indexSender: Sender<CommandIndexJob>,
    sqliteQuerySender: Sender<SqliteJob>,
}

fn handleStream(mut stream: TcpStream) {
    println!("DOIN IT");
    let mut theStr = String::with_capacity(1024 * 2);
    stream.read_to_string(&mut theStr);
    println!("{}",theStr);
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

    let mut f = File::open(&filepath).unwrap();
    let mut contents = String::with_capacity(1024 * 64);
    f.read_to_string(&mut contents);
    let parseRes = parseCommands(&contents);
    for i in parseRes {
        send.send(CommandIndexJob::IndexSource{
            comm: i, context: projectName.clone() });
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
    let listener = TcpListener::bind("127.0.0.1:7777").unwrap();
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                handleStream(stream);
            }
            Err(e) => {
                println!("{}",e);
            }
        }
    }
}

fn main() {
    use rusqlite::SqliteConnection;

    let (txJob,rxJob) = channel::<CommandIndexJob>();
    let (txQuery,rxQuery) = channel::<SqliteJob>();
    let (txEnd,rxEnd) = channel::<i32>();
    let clonedTxJob = txJob.clone();
    let clonedTxQuery = txQuery.clone();
    let inst = MyAppInstance {
        indexSender: txJob,
        sqliteQuerySender: txQuery,
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

    //let cmd = "/home/deividas/Desktop/ramdisk/bld/compile_commands.json".to_string();
    //hitTheFile(cmd,"moo".to_string());
    let jerb = CommandIndexJob::ProcessCompCommands {
        path: "/home/deividas/Desktop/ramdisk/bld/compile_commands.json".to_string(),
        context: "shazzlow".to_string(),
    };
    inst.indexSender.send(jerb);

    // synchronize, one for db
    // other for processing
    std::thread::sleep_ms(50000);
    inst.indexSender.send(CommandIndexJob::Stop);
    inst.sqliteQuerySender.send(SqliteJob::Stop);
    rxEnd.recv();
    rxEnd.recv();

    //listen(inst);
}
