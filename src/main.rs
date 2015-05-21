#![allow(non_snake_case)]

extern crate rustc_serialize;
use std::sync::mpsc::channel;
use std::sync::mpsc::Sender;
use std::thread;
use std::io::prelude::*;
use std::net::{TcpListener, TcpStream};

#[derive(Debug)]
struct Command {
    dir: String,
    command: String,
    file: String,
}

struct CommandIndexJob {
    comm: Command,
    context: String,
}

struct MyAppInstance {
    indexSender: Sender<CommandIndexJob>,
    sqliteQuerySender: Sender<String>,
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
        println!("{:?}",comm);
        commands.push(comm);
    }
    commands
}

fn hitTheFile(filepath: String,projectName: String) {
    use std::io::prelude::*;
    use std::fs::File;

    let mut f = File::open(&filepath).unwrap();
    let mut contents = String::with_capacity(1024 * 64);
    f.read_to_string(&mut contents);
    let parseRes = parseCommands(&contents);
}

fn listen() {
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
    let cmd = "/home/deividas/Desktop/ramdisk/bld/compile_commands.json".to_string();
    hitTheFile(cmd,"moo".to_string());

    let (txJob,rxJob) = channel();
    let (txQuery,rxQuery) = channel();
    let inst = MyAppInstance {
        indexSender: txJob,
        sqliteQuerySender: txQuery,
    };
}
