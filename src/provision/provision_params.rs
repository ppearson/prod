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

use std::collections::BTreeMap;
use std::fmt;
use std::io::{BufRead, BufReader};
use std::path::{Path};

use crate::common::{FileLoadError};

use super::provision_common::{ProvisionActionType, ProvisionResponseWaitType};

#[derive(Clone, Debug, PartialEq)]
pub enum ParamValue {
    StringVal(String),
    StringArray(Vec<String>)
}

impl fmt::Display for ParamValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self {
            ParamValue::StringVal(s) => write!(f, "'{}'", s),
            ParamValue::StringArray(arr) => {
                write!(f, "{{")?;
                for it in arr {
                    write!(f, " {},", it)?;
                }
                write!(f, " }}")
            },
        }
    }
}

#[derive(Clone, Debug)]
pub struct ProvisionParams {
    pub provider:   String,
    pub action:     ProvisionActionType,
    pub wait_type:  ProvisionResponseWaitType,
    pub values:     BTreeMap<String, ParamValue>
}

impl fmt::Display for ProvisionParams {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Provider: {}, Action: {}", self.provider, self.action)?;
        writeln!(f, " params ({}): {{", self.values.len())?;
        for (param, value) in &self.values {
            writeln!(f, "  {}: {}", param, value)?
        }
        writeln!(f, " }}")
    }
}

impl ProvisionParams {
    pub fn new() -> ProvisionParams {
        ProvisionParams { provider: String::new(), action: ProvisionActionType::NotSet,
            wait_type: ProvisionResponseWaitType::WaitForResourceFinalised, values: BTreeMap::new() }
    }

    pub fn from_details(provider: &str, action: ProvisionActionType) -> ProvisionParams {
        ProvisionParams { provider: provider.to_string(), action,
            wait_type: ProvisionResponseWaitType::WaitForResourceFinalised, values: BTreeMap::new() }
    }

    // TODO: something a bit better than this? Not really sure what though? Use a Result to indicate
    //       failure?
    pub fn from_file(path: &str) -> Result<ProvisionParams, FileLoadError> {
        let extension = Path::new(&path).extension().unwrap();
        let mut extension_lower = extension.to_str().unwrap().to_string();
        extension_lower.make_ascii_lowercase();

        if extension_lower == "txt" {
            return ProvisionParams::from_file_txt(path);
        }
        else if extension_lower == "yaml" {
            // TODO:...
        }

        return Err(FileLoadError::CustomError("Unknown file type.".to_string()))
    }

    fn from_file_txt(path: &str) -> Result<ProvisionParams, FileLoadError> {
        let file = std::fs::File::open(path).unwrap();
        let reader = BufReader::new(file);

        // TODO: error handling...

        let mut provision_params = ProvisionParams::new();

        for line in reader.lines() {
            let line = line.unwrap();

            // TODO: might not want to do this when we support hierarchical/multiple actions/items
            let line = line.trim();

            // ignore empty lines and comments
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            if !line.contains(':') {
                eprintln!("Unexpected line in .txt file...");
                continue;
            }

            let string_pairs: Vec<&str> = line.split(':').collect();
            if string_pairs.len() != 2 {
                eprintln!("Unexpected line in .txt file...");
                continue;
            }

            provision_params.ingest_param(string_pairs[0], string_pairs[1].trim());
        }

        return Ok(provision_params);
    }

    fn ingest_param(&mut self, key: &str, val: &str) {
        match key {
            "provider" => {
                self.provider = val.to_string();
            },
            "action" => {
                self.action = match val {
                    "createInstance" => ProvisionActionType::CreateInstance,
                    "deleteInstance" => ProvisionActionType::DeleteInstance,
                    _ => ProvisionActionType::Unknown
//                    _ => ProvisionActionType::Unknown(val.to_string())
                };
            },
            "waitType" => {
                use ProvisionResponseWaitType::*;
                self.wait_type = match val {
                    "returnImmediately" => ReturnImmediatelyAfterAPIRequest,
                    "waitForResourceCreation" => WaitForResourceCreationOrModification,
                    "waitForResourceFinalised" => WaitForResourceFinalised,
                    _ => WaitForResourceCreationOrModification,
                }
            },
            _ => {
                // Note: we promote automatically to StringArray when there's already a key of that name which is a string,
                //       which means we don't overwrite...
                if let Some(param_value) = self.values.get_mut(key) {
                    if let ParamValue::StringArray(array) = param_value {
                        array.push(val.to_string());
                    }
                    else if let ParamValue::StringVal(str) = param_value {
                        // current value is a single string, automatically promote to a string array
                        let str = std::mem::take(str);
                        self.values.insert(key.to_string(), ParamValue::StringArray(vec![str, val.to_string()]));
                    }
                }
                else {
                    // it doesn't exist currently, so add as a new param...
                    self.values.insert(key.to_string(), ParamValue::StringVal(val.to_string()));
                }
            }
        }
    }

    pub fn has_param(&self, key: &str) -> bool {
        return self.values.contains_key(key);
    }

    pub fn get_string_value(&self, key: &str, default: &str) -> String {
        if let Some(ParamValue::StringVal(str)) = self.values.get(key) {
            return str.to_string();
        }
        
        return default.to_string();
    }

    pub fn get_string_value_as_bool(&self, key: &str, default: bool) -> bool {
        if let Some(ParamValue::StringVal(str)) = self.values.get(key) {
            let val = match str.as_str() {
                "0" | "false" => false,
                "1" | "true" => true,
                _ => default
            };

            return val;
        }
        
        return default;
    }

    pub fn get_string_array(&self, key: &str) -> Option<Vec<String>> {
        let val = self.values.get(key);
        if let Some(ParamValue::StringArray(array)) = val {
            return Some(array.clone());
        }
        else if let Some(ParamValue::StringVal(str)) = val {
            // it's currently a single string, but because we've been asked for a string array
            // return the single string as a Vec<String> of that one string...
            return Some(vec![str.to_string()]);
        }

        return None;
    }
}
