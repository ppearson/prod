/*
 Prod
 Copyright 2021-2023 Peter Pearson.
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

#[cfg(feature = "openssh")]
use super::control_connection_openssh::ControlConnectionOpenSSH;

#[cfg(feature = "openssh")]
use ssh2::Session;
use std::fmt;
#[cfg(feature = "openssh")]
use std::net::TcpStream;

#[cfg(feature = "sshrs")]
use super::control_connection_sshrs::ControlConnectionSshRs;
#[cfg(feature = "sshrs")]
use ssh;

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
    target_host:                    String,
    target_port:                    u32,
    user_auth:                      ControlSessionUserAuth,

pub user_type:                      UserType,
pub hide_commands_from_history:     bool  
}

impl ControlSessionParams {
    pub fn new(target_host: &str,
               target_port: u32,
               user_auth: ControlSessionUserAuth,
               hide_commands_from_history: bool) -> ControlSessionParams {
        let user_type = UserType::Standard;
        ControlSessionParams { connection_type: ConnectionType::SSH,
             target_host: target_host.to_string(),
             target_port,
             user_auth,
             user_type,
             hide_commands_from_history }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ControlSessionCreationError {
    ConfigFailure(String),
    ConnectionError(String),
    AuthenticationError(String),
    Other(String),
}

impl fmt::Display for ControlSessionCreationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ConfigFailure(err) => {
                write!(f, "Config Failure: {}", err)
            },
            Self::ConnectionError(err) => {
                write!(f, "Can't connect: {}", err)
            },
            Self::AuthenticationError(err) => {
                write!(f, "Authentication Error: {}", err)
            },
            Self::Other(err) => {
                write!(f, "Other error: {}", err)
            }
        }
    }
}

impl ControlSessionCreationError {
    pub fn should_attempt_connection_retry(&self) -> bool {
        // TODO: think about this some more, it's unlikely to
        //       always be correct, but...

        // for the moment, only return true for connection and other errors.
        match self {
            Self::ConfigFailure(_) => false,
            Self::AuthenticationError(_) => false,
            Self::ConnectionError(_) => true,
            Self::Other(_) => true
        }
    }
}

pub struct ControlSession {
    pub conn:   Box<dyn ControlConnection>, 
    pub params: ControlSessionParams,
}

impl ControlSession {

    #[cfg(feature = "openssh")]
    pub fn new_openssh(control_session_params: ControlSessionParams) -> Result<ControlSession, ControlSessionCreationError> {
        let ssh_host_target = format!("{}:{}", control_session_params.target_host,
                                               control_session_params.target_port );
        let tcp_connection = TcpStream::connect(&ssh_host_target);
        if let Err(err) = tcp_connection {
            return Err(ControlSessionCreationError::ConnectionError(format!("Error connecting to host: {}", err.to_string())));
        }
        let tcp_connection = tcp_connection.unwrap();
        let mut sess = Session::new().unwrap();

        sess.set_tcp_stream(tcp_connection);
        sess.handshake().unwrap();
        let auth_res;
        if let ControlSessionUserAuth::UserPass(user_pass) = &control_session_params.user_auth {
            auth_res = sess.userauth_password(&user_pass.username, &user_pass.password);
            if let Err(err) = auth_res {
                return Err(ControlSessionCreationError::AuthenticationError(
                    format!("Authentication failure with user/pass: {}, err: {}...", &user_pass.username, err)));
            }
        }
        else if let ControlSessionUserAuth::PublicKey(pub_key) = &control_session_params.user_auth {
            let pub_key_path = Some(std::path::Path::new(&pub_key.publickey_path));
            let priv_key_path = std::path::Path::new(&pub_key.privatekey_path);
            auth_res = sess.userauth_pubkey_file(&pub_key.username, pub_key_path,
                                                 priv_key_path, Some(&pub_key.passphrase));
            if let Err(err) = auth_res {
                return Err(ControlSessionCreationError::AuthenticationError(
                    format!("Authentication failure with phrase/key for user: {}, err: {}...", &pub_key.username, err)));
            }
        }
        else {
            // this shouldn't be reach-able, but...
            return Err(ControlSessionCreationError::ConfigFailure("Unhandled auth type".to_string()));
        }

        let ssh_connection = ControlConnectionOpenSSH::new(sess);
        Ok(ControlSession { conn: Box::new(ssh_connection), params: control_session_params })
    }

    #[cfg(feature = "sshrs")]
    pub fn new_sshrs(control_session_params: ControlSessionParams) -> Result<ControlSession, ControlSessionCreationError> {
        let sess_builder;
        
        if let ControlSessionUserAuth::UserPass(user_pass) = &control_session_params.user_auth {
            sess_builder = ssh::create_session().username(&user_pass.username)
                .password(&user_pass.password);
        }
        else if let ControlSessionUserAuth::PublicKey(pub_key) = &control_session_params.user_auth {
            sess_builder = ssh::create_session().username(&pub_key.username)
                .password(&pub_key.passphrase)
                .private_key_path(&pub_key.privatekey_path);
        }
        else {
            // this shouldn't be reach-able, but...
            return Err(ControlSessionCreationError::ConfigFailure("Unhandled auth type".to_string()));
        }

        let ssh_host_target = format!("{}:{}", control_session_params.target_host, control_session_params.target_port);

        let session = sess_builder.connect(&ssh_host_target);
        if let Err(err) = &session {
            match err {
                ssh::SshError::IoError(int_err) => {
                    return Err(ControlSessionCreationError::ConnectionError(format!("Error connecting to host: {}", int_err.to_string())));
                },
                ssh::SshError::TimeoutError => {
                    return Err(ControlSessionCreationError::ConnectionError(format!("Error connecting to host - timed outs: {}", err.to_string())));
                },
                ssh::SshError::AuthError => {
                    return Err(ControlSessionCreationError::AuthenticationError(format!("Error authenticating with host: {}", err.to_string())));
                },
                _           => {
                    return Err(ControlSessionCreationError::ConnectionError(format!("Error connecting to host: {}", err.to_string())));
                }
            }
        }

        let session = session.unwrap();

        let ssh_connection = ControlConnectionSshRs::new(session);
        Ok(ControlSession { conn: Box::new(ssh_connection), params: control_session_params })
    }

    pub fn new_dummy_debug(control_session_params: ControlSessionParams) -> Result<ControlSession, ControlSessionCreationError> {
        let dummy_connection = ControlConnectionDummyDebug::new();

        Ok(ControlSession { conn: Box::new(dummy_connection), params: control_session_params })
    }
}

