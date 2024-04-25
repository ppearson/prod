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

use std::fmt;
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq)]
#[derive(Copy)]
pub enum ProvisionActionType {
    NotSet,
    CreateInstance,
    DeleteInstance,
    Unknown
//    Unknown(String)
}

impl fmt::Display for ProvisionActionType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProvisionActionType::NotSet          => write!(f, "None"),
            ProvisionActionType::CreateInstance  => write!(f, "createInstance"),
            ProvisionActionType::DeleteInstance  => write!(f, "deleteInstance"),
//            ProvisionActionType::Unknown(string) => write!(f, "Unknown('{}')", string)
            ProvisionActionType::Unknown => write!(f, "Unknown")
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
#[derive(Copy)]
pub enum ProvisionResponseWaitType {
    ReturnImmediatelyAfterAPIRequest,
    WaitForResourceCreationOrModification, // wait for an IP address to exist
    WaitForResourceFinalised               // wait for the resource to actually be useable...
}

#[derive(Clone, Debug)]
pub enum ProvisionActionResult {
    NotSupported,
    ErrorNotConfigured(String),
    ErrorMissingParams(String),
    ErrorCantConnect(String),
    ErrorAuthenticationIssue(String),
    Failed(String),
    ActionCreatedInProgress(ActionResultValues),
    ActionCreatedDone(ActionResultValues),
}

#[derive(Clone, Debug)]
pub struct ActionResultValues {
    pub values:     BTreeMap<String, String>
}

impl fmt::Display for ActionResultValues {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, " provision action results ({}): {{", self.values.len())?;
        for (param, value) in &self.values {
            writeln!(f, "  {}: {}", param, value)?
        }
        writeln!(f, " }}")
    }
}

impl ActionResultValues {
    pub fn new() -> ActionResultValues {
        ActionResultValues { values: BTreeMap::new() }
    }

    pub fn has_value(&self, key: &str) -> bool {
        return self.values.contains_key(key);
    }

    pub fn get_value(&self, key: &str, default: &str) -> String {
        let res = self.values.get(key);
        if let Some(str_val) = res {
            return str_val.to_string();
        }
        
        return default.to_string();
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
