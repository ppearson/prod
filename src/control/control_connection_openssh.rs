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

use ssh2::{Session, Channel};

use std::path::Path;

use std::io::BufReader;
use std::io::prelude::*;

use super::control_connection::ControlConnection;

const BUFFER_SIZE: usize = 16 * 1024;

pub struct ControlConnectionOpenSSH {
    pub session:        Session,

    pub prev_std_out:   String,
    pub prev_std_err:   String,

    pub exit_code:      Option<i32>,

    shell_channel:      Option<Channel>,
    have_shell_session: bool,
}

impl ControlConnectionOpenSSH {
    pub fn new(session: Session) -> ControlConnectionOpenSSH {
        ControlConnectionOpenSSH { session, prev_std_out: String::new(), prev_std_err: String::new(),
                                 exit_code: None,
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

        self.prev_std_err = String::new();
        channel.stderr().read_to_string(&mut self.prev_std_err).unwrap();

        channel.wait_close().unwrap();

        if let Ok(code) = channel.exit_status() {
            self.exit_code = Some(code);
        }
        else {
            self.exit_code = None;
        }
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

    
    pub fn get_text_file_contents_via_scp(&self, filepath: &str) -> Result<String, RemoteFileContentsControlError> {
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

    pub fn send_text_file_contents_via_scp(&self, filepath: &str, mode: i32, contents: &str) -> Result<(), RemoteFileContentsControlError> {
        let byte_contents = contents.as_bytes();

        let mut remote_file = self.session.scp_send(Path::new(&filepath), mode, byte_contents.len() as u64, None).unwrap();
        
        // TODO: there seems to be a 32kb limit here in practice based on send_file_via_scp() testing, so this might
        //       need changing to support longer files as well...
        remote_file.write(byte_contents).unwrap();
        // Close the channel and wait for the whole content to be tranferred
        remote_file.send_eof().unwrap();
        remote_file.wait_eof().unwrap();
        remote_file.close().unwrap();
        remote_file.wait_close().unwrap();
        
        return Ok(());
    }

    pub fn send_file_via_scp(&self, local_filepath: &str, dest_filepath: &str, mode: i32) -> Result<(), ()> {
        // TODO: better error handling here and below...
        if !std::path::Path::new(local_filepath).exists() {
            return Err(());
        }

        let file_size = std::fs::metadata(&local_filepath).unwrap().len();

        let mut remote_file = self.session.scp_send(Path::new(dest_filepath), mode, file_size as u64, None).unwrap();

        let mut file = std::fs::File::open(local_filepath).unwrap();
        let mut buffer = Vec::with_capacity(BUFFER_SIZE);
        loop {
            let bytes_read = std::io::Read::by_ref(&mut file).take(BUFFER_SIZE as u64).read_to_end(&mut buffer).unwrap();
            if bytes_read == 0 {
                break;
            }

            let bytes_written = remote_file.write(&buffer);
            if bytes_written.is_ok() {
                let bytes_written = bytes_written.unwrap();
                assert!(bytes_written == bytes_read);

                if bytes_read < BUFFER_SIZE {
                    break;
                }
                
                // buffer is extended each time read_to_end() is called, so we need this.
                // In theory, it should be very cheap, as it doesn't de-allocate the memory...
                buffer.clear();
            }
            else {
                eprintln!("Error writing file to SSH session...");
                return Err(());
            }
        }

        // Close the channel and wait for the whole content to be tranferred
        remote_file.send_eof().unwrap();
        remote_file.wait_eof().unwrap();
        remote_file.close().unwrap();
        remote_file.wait_close().unwrap();

        return Ok(());
    }

    fn receive_file_via_scp(&self, remote_filepath: &str, local_filepath: &str) -> Result<(), ()> {
        let recv_res = self.session.scp_recv(Path::new(&remote_filepath));
        if let Err(err) = recv_res {
            eprintln!("Error opening remote file: code: {}", err.code());
            return Err(());
        }
        
        let (mut remote_file, _stat) = recv_res.unwrap();

        let local_file = std::fs::File::create(local_filepath);
        if let Err(err) = local_file {
            eprintln!("Error creating local file: {}", err.to_string());
            return Err(());
        }
        let mut local_file = local_file.unwrap();

        let mut buffer = Vec::with_capacity(BUFFER_SIZE);
        loop {
            let bytes_read = std::io::Read::by_ref(&mut remote_file).take(BUFFER_SIZE as u64).read_to_end(&mut buffer).unwrap();
            if bytes_read == 0 {
                break;
            }

            let bytes_written = local_file.write(&buffer);
            if bytes_written.is_ok() {
                let bytes_written = bytes_written.unwrap();
                assert!(bytes_written == bytes_read);

                if bytes_read < BUFFER_SIZE {
                    break;
                }
                
                // buffer is extended each time read_to_end() is called, so we need this.
                // In theory, it should be very cheap, as it doesn't de-allocate the memory...
                buffer.clear();
            }
            else {
                eprintln!("Error reading file from SSH session...");
                return Err(());
            }
        }

        // Close the channel and wait for the whole content to be tranferred
        remote_file.send_eof().unwrap();
        remote_file.wait_eof().unwrap();
        remote_file.close().unwrap();
        remote_file.wait_close().unwrap();

        return Ok(());
    }
}

impl ControlConnection for ControlConnectionOpenSSH {

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

    // Note: these methods don't need to be &mut self for the OpenSSH version, but they
    //       do for the SSH-rs version, so...
    fn get_text_file_contents(&mut self, filepath: &str) -> Result<String, RemoteFileContentsControlError> {
        return self.get_text_file_contents_via_scp(filepath);
    }

    fn send_text_file_contents(&mut self, filepath: &str, mode: i32, contents: &str) -> Result<(), RemoteFileContentsControlError> {
        return self.send_text_file_contents_via_scp(filepath, mode, contents);
    }

    fn send_file(&mut self, local_filepath: &str, dest_filepath: &str, mode: i32) -> Result<(), ()> {
        return self.send_file_via_scp(local_filepath, dest_filepath, mode);
    }

    fn receive_file(&mut self, local_filepath: &str, dest_filepath: &str) -> Result<(), ()> {
        return self.receive_file_via_scp(local_filepath, dest_filepath);
    }

}
