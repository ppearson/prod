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

// this is completely superfluous, but is a placeholder for possible different connection strategies in the future...
pub enum ConnectionType {
    SSH
}

#[derive(Clone, Debug, PartialEq)]
pub enum UserType {
    Standard,
    Sudo
}

pub struct ControlSessionUserAuth {
    // TODO: ssh key stuff...

    username:           String,
    password:           String,
}

pub struct ControlSessionParams {
    connection_type:                ConnectionType,
    host_target:                    String,
    user_auth:                      ControlSessionUserAuth,

pub user_type:                      UserType,
pub hide_commands_from_history:     bool  
}

impl ControlSessionParams {
    pub fn new(host_target: &str, username: &str, password: &str, hide_commands_from_history: bool) -> ControlSessionParams {
        let user_type = UserType::Standard;
        ControlSessionParams { connection_type: ConnectionType::SSH, host_target: host_target.to_string(),
                user_auth: ControlSessionUserAuth { username: username.to_string(), password: password.to_string() },
                user_type, hide_commands_from_history }
    }
}

pub struct ControlSession {
    pub conn:   Box<dyn ControlConnection>, 
    pub params: ControlSessionParams,
}

impl ControlSession {

    #[cfg(feature = "ssh")]
    pub fn new_ssh(control_session_params: ControlSessionParams) -> Option<ControlSession> {
        let ssh_host_target = format!("{}:{}", control_session_params.host_target, 22);
        let tcp_connection = TcpStream::connect(&ssh_host_target);
        if tcp_connection.is_err() {
            eprintln!("Error: Can't connect to host: '{}'.", ssh_host_target);
            return None;
        }
        let tcp_connection = tcp_connection.unwrap();
        let mut sess = Session::new().unwrap();

        sess.set_tcp_stream(tcp_connection);
        sess.handshake().unwrap();
        let auth_res = sess.userauth_password(&control_session_params.user_auth.username, &control_session_params.user_auth.password);
        if auth_res.is_err() {
            eprintln!("Error: Authentication failure with user: {}...", control_session_params.user_auth.username);
            return None;
        }

        let ssh_connection = ControlConnectionSSH::new(sess);
        Some(ControlSession { conn: Box::new(ssh_connection), params: control_session_params })
    }

    pub fn new_dummy_debug(control_session_params: ControlSessionParams) -> Option<ControlSession> {
        let dummy_connection = ControlConnectionDummyDebug::new();

        Some(ControlSession { conn: Box::new(dummy_connection), params: control_session_params })
    }
}

