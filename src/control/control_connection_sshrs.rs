/*
 Prod
 Copyright 2021-2025 Peter Pearson.
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

use ssh::{LocalSession, SessionConnector};

use std::fs::File;
use std::path::Path;

use std::io::prelude::*;

use std::net::TcpStream;

use super::control_connection::{ControlConnection, RemoteFileContentsControlError};

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
        if let Err(_err) = exec {
            // TODO: error...
            //   But what?
            return;
        }

        let mut exec = exec.unwrap();
        let result = exec.exec_command(command);
        if let Err(_err) = result {
            // TODO: error...
            return;
        }

        // Note: this is needed here for result processing and for exit_status state to be valid...
        let vec: Vec<u8> = exec.get_output().unwrap();

        if let Ok(exit_code) = exec.exit_status() {
            self.exit_code = Some(exit_code as i32);
        }
        // Note: the output from an ssh-rs exec.send_command() call is not separated into stdout/stderr,
        //       so we have no way of easily identifying if there was an error or not via the stdout/stderr output...
        
        self.prev_std_out = String::from_utf8(vec).unwrap();
        if self.prev_std_out.is_empty() {

        }
    }

    fn send_command_shell(&mut self, _command: &str) {
        
    }

    pub fn get_text_file_contents_via_scp(&mut self, filepath: &str) -> Result<String, RemoteFileContentsControlError> {
        let scp = self.local_session.open_scp();
        if let Err(err) = scp {
            return Err(RemoteFileContentsControlError::CantConnect(err.to_string()));
        }
        let scp = scp.unwrap();
        // use temp-file crate, as there's no current way to get the contents directly with ssr-rs, we need to go
        // via a temp file on disk...
        let tmp_local_file = temp_file::empty();
        let local_temp_file_path = tmp_local_file.path();
        let res = scp.download(local_temp_file_path, Path::new(&filepath));
        if let Err(err) = res {
            return Err(RemoteFileContentsControlError::TransferError(err.to_string()));
        }

        let file_handle = std::fs::File::open(local_temp_file_path);
        if let Ok(mut file) = file_handle {
            let mut file_contents = String::new();

            let read_from_string_res = file.read_to_string(&mut file_contents);
            if let Err(err) = read_from_string_res {
                return Err(RemoteFileContentsControlError::TransferError(err.to_string()));
            }
            
            Ok(file_contents)
        }
        else {
            Err(RemoteFileContentsControlError::CantCreateLocalTempFile(file_handle.err().unwrap().to_string()))
        }
    }

    pub fn send_text_file_contents_via_scp(&mut self, filepath: &str, _mode: i32, contents: &str) -> Result<(), RemoteFileContentsControlError> {
        let scp = self.local_session.open_scp();
        if let Err(err) = scp {
            return Err(RemoteFileContentsControlError::CantConnect(err.to_string()));
        }
        let scp = scp.unwrap();

        // write the text file contents to a temporary file
        let tmp_local_file = temp_file::empty();
        let local_temp_file_path = tmp_local_file.path();
        let local_file = File::create(local_temp_file_path);
        if local_file.is_err() {
            eprintln!("Error creating temporary file to scp text contents to remote: {}", local_temp_file_path.display());
            return Err(RemoteFileContentsControlError::CantCreateLocalTempFile(local_file.err().unwrap().to_string()));
        }
        let mut local_file = local_file.unwrap();
        local_file.write_all(contents.as_bytes()).unwrap();

        // TODO: not sure what to do about the file mode... ssh-rs does not support specifying the mode
        //       via the upload() method, but maybe it copies it from the source file, and we can just
        //       set the required mode locally?

        let res = scp.upload(Path::new(&local_temp_file_path), Path::new(filepath));
        if let Err(err) = res {
            return Err(RemoteFileContentsControlError::TransferError(err.to_string()));
        }
        Ok(())
    }

    pub fn send_file_via_scp(&mut self, local_filepath: &str, dest_filepath: &str, _mode: i32) -> Result<(), ()> {
        // TODO: better error handling here and below...
        if !std::path::Path::new(local_filepath).exists() {
            return Err(());
        }

        let scp = self.local_session.open_scp();
        if let Err(_err) = scp {
            // TODO:
            return Err(());
        }
        let scp = scp.unwrap();
       
        let res = scp.upload(Path::new(&local_filepath), Path::new(dest_filepath));
        if let Err(_err) = res {
            return Err(());
        }

        Ok(())
    }

    fn receive_file_via_scp(&mut self, remote_filepath: &str, local_filepath: &str) -> Result<(), ()> {
        let scp = self.local_session.open_scp();
        if let Err(_err) = scp {
            // TODO:
            return Err(());
        }
        let scp = scp.unwrap();
       
        let res = scp.download(Path::new(&local_filepath), Path::new(remote_filepath));
        if let Err(_err) = res {
            return Err(());
        }

        Ok(())
    }
}

impl ControlConnection for ControlConnectionSshRs {

    fn send_command(&mut self, command: &str) {
//        self.debug(command);
        self.send_command_exec(command);
//        self.send_command_shell(command);
    }

    fn had_command_response(&self) -> bool {
        !self.prev_std_out.is_empty()
    }

    fn get_previous_stdout_response(&self) -> &str {
        &self.prev_std_out
    }

    fn get_previous_stderr_response(&self) -> Option<&str> {
        if self.prev_std_err.is_empty() {
            return None;
        }

        Some(&self.prev_std_err)
    }

    fn get_exit_code(&self) -> Option<i32> {
        self.exit_code
    }

    fn did_exit_with_error_code(&self) -> bool {
        if let Some(ec) = self.exit_code {
            return ec != 0;
        }
        
        false
    }

    fn get_text_file_contents(&mut self, filepath: &str) -> Result<String, RemoteFileContentsControlError> {
        self.get_text_file_contents_via_scp(filepath)
    }

    fn send_text_file_contents(&mut self, filepath: &str, mode: i32, contents: &str) -> Result<(), RemoteFileContentsControlError> {
        self.send_text_file_contents_via_scp(filepath, mode, contents)
    }

    fn send_file(&mut self, local_filepath: &str, dest_filepath: &str, mode: i32) -> Result<(), ()> {
        self.send_file_via_scp(local_filepath, dest_filepath, mode)
    }

    fn receive_file(&mut self, local_filepath: &str, dest_filepath: &str) -> Result<(), ()> {
        self.receive_file_via_scp(local_filepath, dest_filepath)
    }

}
