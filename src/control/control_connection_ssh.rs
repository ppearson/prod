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

use ssh2::{Session, Channel};

use std::path::Path;

use std::io::{BufReader};
use std::io::prelude::*;

use super::control_connection::{ControlConnection};

pub struct ControlConnectionSSH {
    pub session:        Session,

    pub prev_std_out:   String,
    pub prev_std_err:   String,

    shell_channel:      Option<Channel>,
    have_shell_session: bool,
}

impl ControlConnectionSSH {
    pub fn new(session: Session) -> ControlConnectionSSH {
        ControlConnectionSSH { session, prev_std_out: String::new(), prev_std_err: String::new(),
                                 shell_channel: None, have_shell_session: false }
    }

    fn debug(&mut self, command: &str) {
        eprintln!("Command: '{}'", command);
    }

    fn send_command_exec(&mut self, command: &str) {
        // Currently we spawn a new channel for each request, which isn't great...
        let mut channel = self.session.channel_session().unwrap();

        channel.exec(command).unwrap();

        self.prev_std_out = String::new();
        channel.read_to_string(&mut self.prev_std_out).unwrap();
    }

    fn send_command_shell(&mut self, command: &str) {
        if !self.have_shell_session {
            self.session.set_timeout(2000);
            let mut channel = self.session.channel_session().unwrap();

            channel.request_pty("xterm", None, None).unwrap();

            channel.shell().unwrap();

            self.shell_channel = Some(channel);
            self.have_shell_session = true;
        }

        let channel = self.shell_channel.as_mut().unwrap();
        channel.write(command.as_bytes()).unwrap();

        let response = BufReader::new(channel.stream(0));
        let mut response_lines = response.lines();

        while let Some(Ok(line)) = response_lines.next() {
            eprintln!("Resp: {}", line);
        }
    }

    pub fn get_text_file_contents_via_scp(&self, filepath: &str) -> Result<String, ()> {
        let (mut remote_file, _stat) = self.session.scp_recv(Path::new(&filepath)).unwrap();

        let mut byte_contents = Vec::new();
        remote_file.read_to_end(&mut byte_contents).unwrap();

        // Close the channel and wait for the whole content to be tranferred
        remote_file.send_eof().unwrap();
        remote_file.wait_eof().unwrap();
        remote_file.close().unwrap();
        remote_file.wait_close().unwrap();

        let string_contents = String::from_utf8_lossy(&byte_contents);

        return Ok(string_contents.to_string());
    }

    pub fn send_text_file_contents_via_scp(&self, filepath: &str, mode: i32, contents: &str) -> Result<(), ()> {
        let byte_contents = contents.as_bytes();

        let mut remote_file = self.session.scp_send(Path::new(&filepath), mode, byte_contents.len() as u64, None).unwrap();
        remote_file.write(byte_contents).unwrap();
        // Close the channel and wait for the whole content to be tranferred
        remote_file.send_eof().unwrap();
        remote_file.wait_eof().unwrap();
        remote_file.close().unwrap();
        remote_file.wait_close().unwrap();
        
        return Ok(());
    }
}

impl ControlConnection for ControlConnectionSSH {

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

    fn get_text_file_contents(&self, filepath: &str) -> Result<String, ()> {
        return self.get_text_file_contents_via_scp(filepath);
    }

    fn send_text_file_contents(&self, filepath: &str, mode: i32, contents: &str) -> Result<(), ()> {
        return self.send_text_file_contents_via_scp(filepath, mode, contents);
    }

    
}
