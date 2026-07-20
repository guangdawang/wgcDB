// src/server.rs
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};

use wgc_db::{database, Database};

const DB_FILE: &str = "wgcDB";
const WAL_FILE: &str = "wgc_db.wal";

pub fn run(port: u16) {
    let listener = TcpListener::bind(("127.0.0.1", port))
        .unwrap_or_else(|e| panic!("无法绑定端口 {}: {}", port, e));
    println!("WgcDB 服务器已启动，监听端口 {} ...", port);

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                std::thread::spawn(|| handle_client(stream));
            }
            Err(e) => {
                eprintln!("连接错误: {}", e);
            }
        }
    }
}

fn handle_client(stream: TcpStream) {
    let peer = stream.peer_addr().map(|a| a.to_string()).unwrap_or_default();
    println!("新连接: {}", peer);

    let mut reader = BufReader::new(&stream);
    let mut line = String::new();

    if let Err(e) = reader.read_line(&mut line) {
        eprintln!("读取请求失败 ({}): {}", peer, e);
        return;
    }

    let sql = line.trim().to_string();
    if sql.is_empty() {
        return;
    }

    let mut db = match Database::load(DB_FILE) {
        Ok(db) => db,
        Err(e) => {
            let _ = writeln!(&stream, "{{\"error\": \"加载数据库失败: {}\"}}", e);
            return;
        }
    };
    db.set_wal_path(WAL_FILE);

    if let Err(e) = db.recover() {
        let _ = writeln!(&stream, "{{\"error\": \"WAL 恢复失败: {}\"}}", e);
        return;
    }

    let result = wgc_db::execute_sql(&mut db, &sql);
    let response = match result {
        Ok(res) => serde_json::to_string(&res)
            .unwrap_or_else(|e| format!("{{\"error\": \"序列化结果失败: {}\"}}", e)),
        Err(e) => format!("{{\"error\": \"{}\"}}", e),
    };

    if let Err(e) = writeln!(&stream, "{}", response) {
        eprintln!("发送响应失败 ({}): {}", peer, e);
    }

    if let Err(e) = db.save(DB_FILE) {
        eprintln!("保存数据库失败 ({}): {}", peer, e);
    }
    if let Err(e) = database::wal::clear(WAL_FILE) {
        eprintln!("清理 WAL 失败 ({}): {}", peer, e);
    }
}