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

use std::cmp::Ordering;
use std::fmt;
use std::io::{BufReader, Read};
use std::path::{Path};

use yaml_rust::{Yaml, YamlLoader};

use crate::common::{FileLoadError};
use crate::params::{ParamValue, Params};
use super::control_common::{ControlConnection};

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[derive(Copy)]
pub enum ControlActionType {
    NotSet,
    Unrecognised,
    AddUser,
    CreateDirectory,
    PackagesInstall,
    SystemCtl,
    Firewall,
    EditFile,
    CopyPath,
}

impl fmt::Display for ControlActionType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ControlActionType::NotSet           => write!(f, "None"),
            ControlActionType::Unrecognised     => write!(f, "Unrecognised"),
            ControlActionType::AddUser          => write!(f, "addUser"),
            ControlActionType::CreateDirectory  => write!(f, "createDirectory"),
            ControlActionType::PackagesInstall  => write!(f, "packagesInstall"),
            ControlActionType::SystemCtl        => write!(f, "systemCtl"),
            ControlActionType::Firewall         => write!(f, "firewall"),
            ControlActionType::EditFile         => write!(f, "editFile"),
            ControlActionType::CopyPath         => write!(f, "copyPath"),
        }
    }
}
/*
impl Ord for ControlActionType {
    fn cmp(&self, other: &Self) -> Ordering {
        let self_u8 = self as u8;
        let other_u8 = other as u8;
        self_u8.cmp(&other_u8)
    }
}
*/

#[derive(Clone, Debug, PartialEq)]
pub enum ActionResult {
    NotImplemented,
    InvalidParams,
    CantConnect,
    AuthenticationIssue,
    Failed(String),
    Success
}

#[derive(Clone, Debug)]
pub struct ControlActions {
    pub provider:   String,
    pub host:       String,
    pub user:       String,

    pub actions:    Vec<ControlAction>,
}

#[derive(Clone, Debug)]
pub struct ControlAction {
    pub action:     ControlActionType,
    pub params:     Params
}

impl fmt::Display for ControlActions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Provider: {}, Host: {}, User: {},", self.provider, self.host, self.user)?;
        writeln!(f, " actions ({}): {{", self.actions.len())?;
        for action in &self.actions {
            write!(f, "  {}", action)?
        }
        writeln!(f, " }}")
    }
}

impl ControlActions {
    pub fn new() -> ControlActions {
        ControlActions { provider: String::new(), host: String::new(), user: String::new(),
                         actions: Vec::with_capacity(0)}
    }

    // TODO: something a bit better than this? Not really sure what though? Use a Result to indicate
    //       failure?
    pub fn from_file(path: &str) -> Result<ControlActions, FileLoadError> {
        let extension = Path::new(&path).extension().unwrap();
        let mut extension_lower = extension.to_str().unwrap().to_string();
        extension_lower.make_ascii_lowercase();

        if extension_lower == "txt" {
            return ControlActions::from_file_txt(path);
        }
        else if extension_lower == "yaml" {
            return ControlActions::from_file_yaml(path);
        }

        return Err(FileLoadError::CustomError("Unknown file type.".to_string()))
    }

    fn from_file_txt(path: &str) -> Result<ControlActions, FileLoadError> {
        let file = std::fs::File::open(path).unwrap();
        let _reader = BufReader::new(file);

        // TODO: error handling...

        let provision_params = ControlActions::new();

        return Ok(provision_params);
    }

    fn from_file_yaml(path: &str) -> Result<ControlActions, FileLoadError> {
        let mut control_actions = ControlActions::new();

        let file_open_res = std::fs::File::open(path);
        if let Ok(mut file) = file_open_res {
            let mut yaml_content = String::new();

            let read_from_string_res = file.read_to_string(&mut yaml_content);
    
            if read_from_string_res.is_ok() {
                let yaml_load_res = YamlLoader::load_from_str(&yaml_content);
                if yaml_load_res.is_ok() {
                    if let Ok(document) = yaml_load_res {
                        let doc: &Yaml = &document[0];
                        
                        if let yaml_rust::Yaml::Hash(ref hash) = doc {
                            for (key, value) in hash {
                                match key.as_str().unwrap() {
                                    "provider" => {
                                        control_actions.provider = value.as_str().unwrap().to_string();
                                    },
                                    "host" => {
                                        control_actions.host = value.as_str().unwrap().to_string();
                                    },
                                    "user" => {
                                        control_actions.user = value.as_str().unwrap().to_string();
                                    },
                                    "actions" => {
                                        control_actions.ingest_control_actions_yaml_items(&value);
                                    },
                                    _ => {}
                                }
                            }

                            return Ok(control_actions);
                        }
                    }
                }
                else {
                    // it's an error...
                    if let Some(err) = yaml_load_res.err() {
                        // print error
                        eprintln!("Error loading YAML file: {}, with error: {}", path,
                                err);
                    }
                }
            }
        }

        return Err(FileLoadError::CustomError("Error loading file.".to_string()));
    }

    fn ingest_control_actions_yaml_items(&mut self, actions_item: &yaml_rust::yaml::Yaml) {
        if actions_item.is_array() {
            for item in actions_item.as_vec().unwrap() {
                self.ingest_control_actions_yaml_items(item);
            }
        }
        else {
            // hopefully, it's a hash/map
            if let yaml_rust::Yaml::Hash(ref hash) = actions_item {
                for (key, value) in hash {
                    if let Some(key_str) = key.as_str() {
                        if let yaml_rust::Yaml::Hash(ref val_hash) = value {
                            // it's hopefully an action item
                            self.ingest_control_yaml_action_item(key_str, val_hash);
                        }
                    }
                }
            }
        }
    }

    fn ingest_control_yaml_action_item(&mut self, name: &str, values: &yaml_rust::yaml::Hash) {
        let mut new_action = ControlAction::new();
        // TODO: do this properly, with a registry which maps the name to the Impl derived item...

        new_action.action = match name {
            "addUser" =>            ControlActionType::AddUser,
            "createDirectory" =>    ControlActionType::CreateDirectory,
            "packagesInstall" =>    ControlActionType::PackagesInstall,
            "systemCtl" =>          ControlActionType::SystemCtl,
            "firewall" =>           ControlActionType::Firewall,
            "editFile" =>           ControlActionType::EditFile,
            "copyPath" =>           ControlActionType::CopyPath,
            _ =>                    ControlActionType::Unrecognised
        };

        if new_action.action == ControlActionType::Unrecognised {
            eprintln!("Error: Unrecognised Control Action: '{}', ignoring.", new_action.action);
            return;
        }

        for (key, value) in values {
            if let Some(key_str) = key.as_str() {
                new_action.params.values.insert(key_str.to_string(), ParamValue::from(value.clone()));
            }
        }

        self.actions.push(new_action);
    }
}

impl fmt::Display for ControlAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, " Action: {}, params ({}): {{", self.action, self.params.values.len())?;
        for (param, value) in &self.params.values {
            writeln!(f, "  {}: {}", param, value)?
        }
        writeln!(f, " }}")
    }
}

impl ControlAction {
    pub fn new() -> ControlAction {
        ControlAction { action: ControlActionType::NotSet, params: Params::new() }
    }
}

pub trait ActionProvider {

    // not sure about this one - ideally it'd be static, but...
    fn name(&self) -> String {
        return "".to_string();
    }

    fn add_user(&self, _connection: &mut ControlConnection, _params: &ControlAction) -> ActionResult {
        return ActionResult::NotImplemented;
    }

    fn create_directory(&self, _connection: &mut ControlConnection, _params: &ControlAction) -> ActionResult {
        return ActionResult::NotImplemented;
    }

    fn install_packages(&self, _connection: &mut ControlConnection, _params: &ControlAction) -> ActionResult {
        return ActionResult::NotImplemented;
    }

    fn systemctrl(&self, _connection: &mut ControlConnection, _params: &ControlAction) -> ActionResult {
        return ActionResult::NotImplemented;
    }

    fn firewall(&self, _connection: &mut ControlConnection, _params: &ControlAction) -> ActionResult {
        return ActionResult::NotImplemented;
    }

    fn edit_file(&self, _connection: &mut ControlConnection, _params: &ControlAction) -> ActionResult {
        return ActionResult::NotImplemented;
    }

    fn copy_path(&self, _connection: &mut ControlConnection, _params: &ControlAction) -> ActionResult {
        return ActionResult::NotImplemented;
    }

}
