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

use std::io::{BufReader};
use std::io::prelude::*;

pub struct ControlConnection {
    pub conn:   SSHControl
}

impl ControlConnection {
    pub fn new(session: Session) -> ControlConnection {
        ControlConnection { conn: SSHControl::new(session) }
    }
}

pub struct SSHControl {
    pub session:        Session,

    pub prev_std_out:   String,
    pub prev_std_err:   String,

    shell_channel:      Option<Channel>,
    have_shell_session: bool,
}

impl SSHControl {
    pub fn new(session: Session) -> SSHControl {
        SSHControl { session, prev_std_out: String::new(), prev_std_err: String::new(),
                     shell_channel: None, have_shell_session: false }
    }

    pub fn send_command(&mut self, command: &str) {
//        self.debug(command);
        self.send_command_exec(command);
//        self.send_command_shell(command);
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

    pub fn had_response(&self) -> bool {
        return !self.prev_std_out.is_empty();
    }
}
