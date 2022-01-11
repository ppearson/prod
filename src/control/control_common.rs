/*
 Prod
 Copyright 2021-2022 Peter Pearson.
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
#[cfg(feature = "ssh")]
use std::net::TcpStream;

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

#[derive(Clone, Debug)]
pub struct UserAuthUserPass {
    pub username:           String,
    pub password:           String,
}

impl UserAuthUserPass {
    pub fn new(user: &str, pass: &str) -> UserAuthUserPass {
        return UserAuthUserPass { username: user.to_string(), password: pass.to_string() };
    }
}

#[derive(Clone, Debug)]
pub struct UserAuthPublicKey {
    pub username:           String,
    pub publickey_path:     String,
    pub privatekey_path:    String,
    pub passphrase:         String,
}

impl UserAuthPublicKey {
    // TODO: pass in as String here so this is less verbose?
    pub fn new(username: &str, publickey_path: &str, privatekey_path: &str, passphrase: &str) -> UserAuthPublicKey {
        return UserAuthPublicKey { username: username.to_string(), publickey_path: publickey_path.to_string(),
                                   privatekey_path: privatekey_path.to_string(), passphrase: passphrase.to_string() }
    }
}

#[derive(Clone, Debug)]
pub enum ControlSessionUserAuth {
    UserPass(UserAuthUserPass),
    PublicKey(UserAuthPublicKey)
}

pub struct ControlSessionParams {
    connection_type:                ConnectionType,
    host_target:                    String,
    user_auth:                      ControlSessionUserAuth,

pub user_type:                      UserType,
pub hide_commands_from_history:     bool  
}

impl ControlSessionParams {
    pub fn new(host_target: &str, user_auth: ControlSessionUserAuth, hide_commands_from_history: bool) -> ControlSessionParams {
        let user_type = UserType::Standard;
        ControlSessionParams { connection_type: ConnectionType::SSH, host_target: host_target.to_string(),
                user_auth,
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
        let auth_res;
        if let ControlSessionUserAuth::UserPass(user_pass) = &control_session_params.user_auth {
            auth_res = sess.userauth_password(&user_pass.username, &user_pass.password);
            if auth_res.is_err() {
                eprintln!("Error: Authentication failure with user: {}...", &user_pass.username);
                return None;
            }
        }
        else if let ControlSessionUserAuth::PublicKey(pub_key) = &control_session_params.user_auth {
            let pub_key_path = Some(std::path::Path::new(&pub_key.publickey_path));
            let priv_key_path = std::path::Path::new(&pub_key.privatekey_path);
            auth_res = sess.userauth_pubkey_file(&pub_key.username, pub_key_path,
                                                 priv_key_path, Some(&pub_key.passphrase));
            if auth_res.is_err() {
                eprintln!("Error: Authentication failure with phrase/key for user: {}...", &pub_key.username);
                return None;
            }
        }

        let ssh_connection = ControlConnectionSSH::new(sess);
        Some(ControlSession { conn: Box::new(ssh_connection), params: control_session_params })
    }

    pub fn new_dummy_debug(control_session_params: ControlSessionParams) -> Option<ControlSession> {
        let dummy_connection = ControlConnectionDummyDebug::new();

        Some(ControlSession { conn: Box::new(dummy_connection), params: control_session_params })
    }
}

