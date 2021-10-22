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

use super::provision_common::{ProvisionActionType};

#[derive(Clone, Debug)]
pub struct ProvisionParams {
    pub provider:   String,
    pub action:     ProvisionActionType,
    pub values:     BTreeMap<String, String>
}

impl fmt::Display for ProvisionParams {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Provider: {}, Action: {}\n", self.provider, self.action)?;
        write!(f, " params ({}): {{\n", self.values.len())?;
        for (param, value) in &self.values {
            write!(f, "  {}: {}\n", param, value)?
        }
        write!(f, " }}\n")
    }
}

impl ProvisionParams {
    pub fn new() -> ProvisionParams {
        ProvisionParams { provider: String::new(), action: ProvisionActionType::NotSet, values: BTreeMap::new() }
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

            provision_params.ingest_param(&string_pairs[0], &string_pairs[1].trim());
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
                    _ => ProvisionActionType::Unknown
//                    _ => ProvisionActionType::Unknown(val.to_string())
                };
            }
            _ => {
                self.values.insert(key.to_string(), val.to_string());
            }
        }
    }

    pub fn has_value(&self, key: &str) -> bool {
        return self.values.contains_key(key);
    }

    pub fn get_value(&self, key: &str, default: &str) -> String {
        let res = self.values.get(key);
        let val = match res {
            Some(str_val) => str_val.to_string(),
            _ => default.to_string()
        };

        val.to_string()
    }

    pub fn get_value_as_bool(&self, key: &str, default: bool) -> bool {
        let res = self.values.get(key);
        let val = match res {
            Some(str_val) => {
                match str_val.as_str() {
                    "0" | "false" => false,
                    "1" | "true" => true,
                    _ => default
                }
            },
            _ => default
        };

        val
    }
}
