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

use std::io;

#[derive(Debug)]
pub enum FileLoadError {
    CustomError(String),
    StdError(String),
    IOError(io::Error),
}

impl From<io::Error> for FileLoadError {
    fn from(error: io::Error) -> Self {
        FileLoadError::IOError(error)
    }
}

impl From<std::str::Utf8Error> for FileLoadError {
    fn from(error: std::str::Utf8Error) -> Self {
        FileLoadError::StdError(error.to_string())
    }
}
