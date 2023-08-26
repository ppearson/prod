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

use ssh_rs::ssh;
use ssh_rs::{LocalSession, SessionConnector};

use std::path::Path;

use std::io::{BufReader};
use std::io::prelude::*;

use std::net::TcpStream;

use super::control_connection::{ControlConnection};

const BUFFER_SIZE: usize = 16 * 1024;

pub struct ControlConnectionSshRs {
    local_session:      LocalSession<TcpStream>,

    pub prev_std_out:   String,
    pub prev_std_err:   String,

    pub exit_code:      Option<i32>,
}

impl ControlConnectionSshRs {
    pub fn new(session: SessionConnector<TcpStream>) -> ControlConnectionSshRs {
        ControlConnectionSshRs { local_session: session.run_local(),
                                 prev_std_out: String::new(), prev_std_err: String::new(),
                                 exit_code: None }
    }

    fn debug(&mut self, command: &str) {
        eprintln!("Command: '{}'", command);
    }

    fn send_command_exec(&mut self, command: &str) {
        self.prev_std_out = String::new();
        self.prev_std_err = String::new();
        
        let exec = self.local_session.open_exec();
        if let Err(err) = exec {
            // TODO: error...
            return;
        }

        let exec = exec.unwrap();
        let result = exec.send_command(command);
        if let Err(err) = result {
            return;
        }
        // Note: this is stdout only...
        let vec: Vec<u8> = result.unwrap();
        self.prev_std_out = String::from_utf8(vec).unwrap();
    }

    fn send_command_shell(&mut self, command: &str) {
        
    }

    pub fn get_text_file_contents_via_scp(&mut self, filepath: &str) -> Result<String, ()> {
        let scp = self.local_session.open_scp();
        if let Err(err) = scp {
            // TODO:
            return Err(());
        }
        let scp = scp.unwrap();
        // use temp-file crate, as there's no current way to get the contents directly with ssr-rs, we need to go
        // via a temp file on disk...
        let tmp_local_file = temp_file::empty();
        let local_temp_file_path = tmp_local_file.path();
        let res = scp.download(local_temp_file_path, Path::new(&filepath));
        if let Err(err) = res {
            return Err(());
        }
        // TODO: read file contents...
        return Err(());
    }

    pub fn send_text_file_contents_via_scp(&mut self, filepath: &str, mode: i32, contents: &str) -> Result<(), ()> {
  
        return Err(());
    }

    pub fn send_file_via_scp(&self, local_filepath: &str, dest_filepath: &str, mode: i32) -> Result<(), ()> {
        // TODO: better error handling here and below...
        if !std::path::Path::new(local_filepath).exists() {
            return Err(());
        }

        let file_size = std::fs::metadata(&local_filepath).unwrap().len();

       

        return Ok(());
    }

    fn receive_file_via_scp(&self, remote_filepath: &str, local_filepath: &str) -> Result<(), ()> {
       

        return Ok(());
    }
}

impl ControlConnection for ControlConnectionSshRs {

    fn send_command(&mut self, command: &str) {
//        self.debug(command);
        self.send_command_exec(command);
//        self.send_command_shell(command);
    }

    fn had_command_response(&self) -> bool {
        return !self.prev_std_out.is_empty();
    }

    fn get_previous_stdout_response(&self) -> &str {
        return &self.prev_std_out;
    }

    fn get_previous_stderr_response(&self) -> Option<&str> {
        if self.prev_std_err.is_empty() {
            return None;
        }

        return Some(&self.prev_std_err);
    }

    fn get_exit_code(&self) -> Option<i32> {
        return self.exit_code;
    }

    fn did_exit_with_error_code(&self) -> bool {
        if let Some(ec) = self.exit_code {
            return ec != 0;
        }
        
        return false;
    }

    fn get_text_file_contents(&mut self, filepath: &str) -> Result<String, ()> {
        return self.get_text_file_contents_via_scp(filepath);
    }

    fn send_text_file_contents(&mut self, filepath: &str, mode: i32, contents: &str) -> Result<(), ()> {
        return self.send_text_file_contents_via_scp(filepath, mode, contents);
    }

    fn send_file(&self, local_filepath: &str, dest_filepath: &str, mode: i32) -> Result<(), ()> {
        return self.send_file_via_scp(local_filepath, dest_filepath, mode);
    }

    fn receive_file(&self, local_filepath: &str, dest_filepath: &str) -> Result<(), ()> {
        return self.receive_file_via_scp(local_filepath, dest_filepath);
    }

}
