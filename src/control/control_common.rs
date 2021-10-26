/*
 Prod
 Copyright 2021 Peter Pearson.
 Licensed under the Apache License, Version 2.0 (the "License");
 You may not use this file except in compliance with the License.
 You may obtain a copy of the License at
 http://www.apache.org/licenses/LICENSE-2.0
 Unless required by applicable law or agreed to in writing, software
 distributed under the License is distributed on an "AS IS" BASIS,
 WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 See the License for the specific language governing permissions and
 limitations under the License.
 ---------
*/
#![allow(dead_code)]

use super::control_connection::{ControlConnection, ControlConnectionDummyDebug};

#[cfg(feature = "ssh")]
use super::control_connection_ssh::{ControlConnectionSSH};

#[cfg(feature = "ssh")]
use ssh2::Session;
use std::net::TcpStream;

// #[cfg(feature = "ssh")]
// mod internal {
//     pub fn send_command() {}
// }

// #[cfg(not(feature = "ssh"))]
// mod internal {
//    pub fn send_command() {}
// }

// pub use internal::*;


pub struct ControlSession {
    pub conn:   Box<dyn ControlConnection>, 
}

impl ControlSession {


    #[cfg(feature = "ssh")]
    pub fn new_ssh(host_target: &str, username: &str, password: &str) -> Option<ControlSession> {
        let ssh_host_target = format!("{}:{}", host_target, 22);
        let tcp_connection = TcpStream::connect(&ssh_host_target);
        if tcp_connection.is_err() {
            eprintln!("Error: Can't connect to host: '{}'.", ssh_host_target);
            return None;
        }
        let tcp_connection = tcp_connection.unwrap();
        let mut sess = Session::new().unwrap();

        sess.set_tcp_stream(tcp_connection);
        sess.handshake().unwrap();
        let auth_res = sess.userauth_password(&username, &password);
        if auth_res.is_err() {
            eprintln!("Error: Authentication failure with user: {}...", username);
            return None;
        }

        let ssh_connection = ControlConnectionSSH::new(sess);

        Some(ControlSession { conn: Box::new(ssh_connection) })
    }

    pub fn new_dummy_debug() -> Option<ControlSession> {
        let dummy_connection = ControlConnectionDummyDebug::new();
        Some(ControlSession { conn: Box::new(dummy_connection) })
    }
}

