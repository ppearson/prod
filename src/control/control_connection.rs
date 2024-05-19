/*
 Prod
 Copyright 2021-2024 Peter Pearson.
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

use super::control_actions::ControlAction;

#[derive(Clone, Debug)]
pub enum RemoteFileContentsControlError {
    NotImplemented,
    LocalFileDoesntExist(String),
    RemoteFileDoesntExist(String),
    CantConnect(String),
    AuthenticationIssue(String),
    CantCreateLocalTempFile(String), // not relevant for all ControlConnection impls...
    TransferError(String),
    Other(String),
}

pub trait ControlConnection {
    fn send_command(&mut self, _command: &str) {

    }

    // whether there was a response output to stdout...
    fn had_command_response(&self) -> bool {
        false
    }

    fn get_previous_stdout_response(&self) -> &str {
        ""
    }

    fn get_previous_stderr_response(&self) -> Option<&str> {
        None
    }

    fn get_exit_code(&self) -> Option<i32> {
        None
    }

    fn did_exit_with_error_code(&self) -> bool {
        false
    }

    // helper to return a generic error string when a remote command returns an error exit_code, including the command run,
    // and any stderr output if that is found (the ssh-rs backend doesn't support extracting stderr output vs stdout).
    fn return_failed_command_error_response_str(&self, command_string: &str, control_action: &ControlAction) -> String {
        // if there's an stderr response, also include that
        if let Some(std_err_response) = self.get_previous_stderr_response() {
            return format!("Unexpected error exit code after running command: '{}' within control method: '{}', with stderr response: {}", command_string,
                            control_action.action, std_err_response);
        }
        else {
            // we don't have any stderr response (either because there wasn't any, or the control connection
            // backend didn't support getting it, i.e. ssh-rs backend)
            return format!("Unexpected error exit code after running command: '{}' within control method: '{}'.", command_string,
                            control_action.action);
        }
    }

    fn get_text_file_contents(&mut self, _filepath: &str) -> Result<String, RemoteFileContentsControlError> {
        Err(RemoteFileContentsControlError::NotImplemented)
    }

    fn send_text_file_contents(&mut self, _filepath: &str, _mode: i32, _contents: &str) -> Result<(), RemoteFileContentsControlError> {
        Err(RemoteFileContentsControlError::NotImplemented)
    }

    fn send_file(&mut self, _local_filepath: &str, _dest_filepath: &str, _mode: i32) -> Result<(), ()> {
        Err(())
    }

    fn receive_file(&mut self, _remote_filepath: &str, _local_filepath: &str) -> Result<(), ()> {
        Err(())
    }
}

pub struct ControlConnectionDummyDebug {

}

impl ControlConnectionDummyDebug {
    pub fn new() -> ControlConnectionDummyDebug {
        ControlConnectionDummyDebug {  }
    }
}

impl ControlConnection for ControlConnectionDummyDebug {

    fn send_command(&mut self, command: &str) {
        eprintln!("Running command: '{}'", command);
    }

    fn had_command_response(&self) -> bool {
        false
    }

    fn get_previous_stdout_response(&self) -> &str {
        ""
    }

    fn get_text_file_contents(&mut self, _filepath: &str) -> Result<String, RemoteFileContentsControlError> {
        Err(RemoteFileContentsControlError::NotImplemented)
    }

    fn send_text_file_contents(&mut self, _filepath: &str, _mode: i32, _contents: &str) -> Result<(), RemoteFileContentsControlError> {
        Err(RemoteFileContentsControlError::NotImplemented)
    }
}
