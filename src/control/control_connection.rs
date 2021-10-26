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

pub trait ControlConnection {
    fn send_command(&mut self, _command: &str) {

    }

    fn had_command_response(&self) -> bool {
        return false;
    }

    fn get_previous_stdout_response(&self) -> &str {
        return "";
    }

    fn get_text_file_contents(&self, _filepath: &str) -> Result<String, ()> {
        return Err(());
    }

    fn send_text_file_contents(&self, _filepath: &str, _mode: i32, _contents: &str) -> Result<(), ()> {
        return Err(());
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
        return false;
    }

    fn get_previous_stdout_response(&self) -> &str {
        return "";
    }

    fn get_text_file_contents(&self, _filepath: &str) -> Result<String, ()> {
        return Err(());
    }

    fn send_text_file_contents(&self, _filepath: &str, _mode: i32, _contents: &str) -> Result<(), ()> {
        return Err(());
    }
}
