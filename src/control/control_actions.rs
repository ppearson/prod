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
use std::io::{BufReader, Read};
use std::path::Path;

use yaml_rust::{Yaml, YamlLoader};

use crate::common::FileLoadError;
use crate::control::control_common::UserAuthPublicKey;
use crate::control::control_system_validation::SystemValidation;
use crate::params::{ParamValue, Params};
use super::control_common::{ControlSession, ControlSessionUserAuth, UserAuthUserPass};
use super::control_common::{ControlSessionParams, UserType};

// Note: try and keep the convention of <action><item>
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[derive(Copy)]
pub enum ControlActionType {
    NotSet,
    Unrecognised,
    GenericCommand,
    AddUser,
    CreateDirectory,
    RemoveDirectory,
    InstallPackages,
    RemovePackages,
    SystemCtl,  // TODO: rename this to systemd?
    Firewall,
    EditFile,
    CopyPath,
    RemoveFile,
    DownloadFile,
    TransmitFile,
    ReceiveFile,
    CreateSymlink,
    SetTimeZone,
    DisableSwap,
    CreateFile,
    AddGroup,
    SetHostname,
    CreateSystemdService,
}

impl fmt::Display for ControlActionType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ControlActionType::NotSet               => write!(f, "None"),
            ControlActionType::Unrecognised         => write!(f, "Unrecognised"),
            ControlActionType::GenericCommand       => write!(f, "genericCommand"),
            ControlActionType::AddUser              => write!(f, "addUser"),
            ControlActionType::CreateDirectory      => write!(f, "createDirectory"),
            ControlActionType::RemoveDirectory      => write!(f, "removeDirectory"),
            ControlActionType::InstallPackages      => write!(f, "installPackages"),
            ControlActionType::RemovePackages       => write!(f, "removePackages"),
            ControlActionType::SystemCtl            => write!(f, "systemCtl"),
            ControlActionType::Firewall             => write!(f, "firewall"),
            ControlActionType::EditFile             => write!(f, "editFile"),
            ControlActionType::CopyPath             => write!(f, "copyPath"),
            ControlActionType::RemoveFile           => write!(f, "removeFile"),
            ControlActionType::DownloadFile         => write!(f, "downloadFile"),
            ControlActionType::TransmitFile         => write!(f, "transmitFile"),
            ControlActionType::ReceiveFile          => write!(f, "receiveFile"),
            ControlActionType::CreateSymlink        => write!(f, "createSymlink"),
            ControlActionType::SetTimeZone          => write!(f, "setTimeZone"),
            ControlActionType::DisableSwap          => write!(f, "disableSwap"),
            ControlActionType::CreateFile           => write!(f, "createFile"),
            ControlActionType::AddGroup             => write!(f, "addGroup"),
            ControlActionType::SetHostname          => write!(f, "setHostname"),
            ControlActionType::CreateSystemdService => write!(f, "createSystemdService"),
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
pub enum ActionError {
    NotImplemented,
    InvalidParams(String),
    CantConnect,
    AuthenticationIssue,
    FailedCommand(String),
    FailedOther(String),
}

#[derive(Clone, Debug)]
pub struct ControlActions {
    // provider to use
    pub provider:   String,
    // hostname to connect to
    pub hostname:   String,
    // port to use 
    pub port:       Option<u32>,

    // authentication
    pub auth:       ControlSessionUserAuth,

    // optional validation
    pub system_validation: SystemValidation,

    // full actions to run
    pub actions:    Vec<ControlAction>,
}

#[derive(Clone, Debug)]
pub struct ControlAction {
    pub action:     ControlActionType,
    pub params:     Params
}

impl fmt::Display for ControlActions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Provider: {}, Hostname: {}, User: {},", self.provider, self.hostname, "")?; // TODO: fix this for auth
        writeln!(f, " actions ({}): {{", self.actions.len())?;
        for action in &self.actions {
            write!(f, "  {}", action)?
        }
        writeln!(f, " }}")
    }
}

impl ControlActions {
    pub fn new() -> ControlActions {
        ControlActions { provider: String::new(),
                         hostname: String::new(),
                         port: None,
                         auth: ControlSessionUserAuth::UserPass(UserAuthUserPass::new("", "")),
                         system_validation: SystemValidation::new(),
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

        Err(FileLoadError::CustomError("Unknown file type.".to_string()))
    }

    fn from_file_txt(path: &str) -> Result<ControlActions, FileLoadError> {
        let file = std::fs::File::open(path).unwrap();
        let _reader = BufReader::new(file);

        // TODO: 

        let provision_params = ControlActions::new();

        Ok(provision_params)
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

                            let auth_params = process_auth_yaml_items(hash);
                            if auth_params.is_none() {
                                // 
                                eprintln!("Error: couldn't work out auth/user settings for control params");
                                return Err(FileLoadError::CustomError("Error loading file.".to_string()));
                            }

                            // otherwise, assume it's okay
                            control_actions.auth = auth_params.unwrap();

                            for (key, value) in hash {
                                match key.as_str().unwrap() {
                                    "provider" => {
                                        control_actions.provider = value.as_str().unwrap().to_string();
                                    },
                                    // TODO: still support "host" for backwards-compatibility for the moment, but at some point remove it...
                                    "host" | "hostname" => {
                                        control_actions.hostname = value.as_str().unwrap().to_string();
                                    },
                                    "port" => {
                                        match value.clone() {
                                            Yaml::Integer(val) => {
                                                control_actions.port = Some(val as u32);
                                            },
                                            _ => {
                                                eprintln!("Error parsing 'port' param as a string: input YAML value was of an unexpected type.");
                                                return Err(FileLoadError::CustomError("Error loading file.".to_string()));
                                            }
                                        }
                                    },
                                    "systemValidation" => {
                                        // For "convenience", we allow different things, so parse it into a string,
                                        // but note that in Yaml its type could be a string or an integer...
                                        // TODO: Supporting things like "20.04" without being quoted in YAML might get annoying...
                                        //       I'd assume it'd likely be interpreted by YAML as a Real/float, and loose the leading '0' ?
                                        let value_as_string = match value.clone() {
                                            Yaml::String(val) => {
                                                val.clone()
                                            },
                                            Yaml::Integer(val) => {
                                                format!("{}", val)
                                            },
                                            _ => {
                                                eprintln!("Error parsing 'systemValidation' param as a string: input YAML value was of an unexpected type.");
                                                return Err(FileLoadError::CustomError("Error loading file.".to_string()));
                                            }
                                        };

                                        let parse_result = SystemValidation::parse_string_value(&value_as_string);
                                        if let Ok(validation) = parse_result {
                                            control_actions.system_validation = validation;
                                        }
                                        else if let Err(err) = parse_result {
                                            eprintln!("Error parsing 'systemValidation' param: {}", err);
                                            return Err(FileLoadError::CustomError("Error loading file.".to_string()));
                                        }
                                    }
                                    "actions" => {
                                        control_actions.ingest_control_actions_yaml_items(value);
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

        Err(FileLoadError::CustomError("Error loading file.".to_string()))
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
            "genericCommand" =>         ControlActionType::GenericCommand,
            "addUser" =>                ControlActionType::AddUser,
            "createDirectory" =>        ControlActionType::CreateDirectory,
            "removeDirectory" =>        ControlActionType::RemoveDirectory,
            "installPackages" =>        ControlActionType::InstallPackages,
            "removePackages" =>         ControlActionType::RemovePackages,
            "systemCtl" =>              ControlActionType::SystemCtl,
            "firewall" =>               ControlActionType::Firewall,
            "editFile" =>               ControlActionType::EditFile,
            "copyPath" =>               ControlActionType::CopyPath,
            "removeFile" =>             ControlActionType::RemoveFile,
            "downloadFile" =>           ControlActionType::DownloadFile,
            "transmitFile" =>           ControlActionType::TransmitFile,
            "receiveFile" =>            ControlActionType::ReceiveFile,
            "createSymlink" =>          ControlActionType::CreateSymlink,
            "setTimeZone" =>            ControlActionType::SetTimeZone,
            "disableSwap" =>            ControlActionType::DisableSwap,
            "createFile" =>             ControlActionType::CreateFile,
            "addGroup" =>               ControlActionType::AddGroup,
            "setHostname" =>            ControlActionType::SetHostname,
            "createSystemdService" =>   ControlActionType::CreateSystemdService,
            _ =>                        ControlActionType::Unrecognised
        };

        if new_action.action == ControlActionType::Unrecognised {
            eprintln!("Error: Unrecognised Control Action: '{}', ignoring.", name);
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

fn get_yaml_map_item_as_string(map: &yaml_rust::yaml::Hash, str_val: &str) -> Option<String> {
    if let Some(item) = map.get(&Yaml::String(str_val.to_string())) {
        if let Some(item_str) = item.as_str() {
            return Some(item_str.to_string());
        }
    }

    None
}

// Note: this passes through the raw values-as is, we don't do any replacement
//       or prompting the user at this stage...
// TODO: maybe return a Result/Err from this, so we get better feedback when there's
//       a problem?
fn process_auth_yaml_items(map: &yaml_rust::yaml::Hash) -> Option<ControlSessionUserAuth> {
    // TODO: this is a mess, the YAML handling is really awkward...

    #[derive(PartialEq)]
    enum AuthType {
        Unknown,
        UserPass,
        PublicKey
    }

    let mut auth_type = AuthType::Unknown;

    // if we have an 'authType', that takes precedence...
    let auth_type_param = get_yaml_map_item_as_string(map, "authType");
    if let Some(auth_type_param_str) = auth_type_param {
        if "userpass".eq_ignore_ascii_case(&auth_type_param_str) {
            auth_type = AuthType::UserPass; // redundant currently, but...
        }
        else if "publickey".eq_ignore_ascii_case(&auth_type_param_str) {
            auth_type = AuthType::PublicKey;
        }
        else {
            eprintln!("Error: unrecognised control command 'authType' param: '{}'", auth_type_param_str);
            return None;
        }
    }

    // TODO: this can likely be re-done in a less duplicate way, but for the moment, just
    //       get things working...
    if auth_type == AuthType::Unknown {
        // we don't know the auth type, so try and detect it from the params which are present

        let is_pubkey = map.contains_key(&Yaml::String("publicKeyPath".to_string())) ||
                            map.contains_key(&Yaml::String("privateKeyPath".to_string())) ||
                            map.contains_key(&Yaml::String("passphrase".to_string()));
        if is_pubkey {
            auth_type = AuthType::PublicKey;
        }
        else {
            auth_type = AuthType::UserPass;
        }
    }

    let username = get_yaml_map_item_as_string(map, "user").unwrap_or("$PROMPT".to_string());

    if auth_type == AuthType::UserPass {
        
        let password = get_yaml_map_item_as_string(map, "password").unwrap_or_default();

        return Some(ControlSessionUserAuth::UserPass(UserAuthUserPass::new(&username, &password)));
    }
    else if auth_type == AuthType::PublicKey {
        
        // for the moment, check key params are described
        if !map.contains_key(&Yaml::String("publicKeyPath".to_string())) ||
                !map.contains_key(&Yaml::String("privateKeyPath".to_string())) {
            
            eprintln!("Error: Invalid user auth credentials supplied to control command params. Incomplete key path details were provided.");
            eprintln!("   Check that the 'publicKeyPath' and 'privateKeyPath' params are supplied if you want to use ssh key authentication.");

            return None;
        }

        let public_key = get_yaml_map_item_as_string(map, "publicKeyPath").unwrap();
        let private_key = get_yaml_map_item_as_string(map, "privateKeyPath").unwrap();

        if public_key.is_empty() || private_key.is_empty() {
            eprintln!("Error: Check that the 'publicKeyPath' and 'privateKeyPath' params are supplied as auth credentials.");
            return None;
        }

        let passphrase = get_yaml_map_item_as_string(map, "passphrase").unwrap_or_default();

        return Some(ControlSessionUserAuth::PublicKey(UserAuthPublicKey::new(&username, &public_key, &private_key, &passphrase)));
    }
    else {
        // this shouldn't be possible, but...
        return None;
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

    // convenience method to get an action parameter which is required as a string, and if it doesn't
    // exist, it returns an Err ActionResult::InvalidParams for convenient fall-through...
    pub fn get_required_string_param(&self, param_name: &str) -> Result<String, ActionError> {
        if let Some(value) = self.params.get_string_value(param_name) {
            Ok(value)
        }
        else {
            Err(ActionError::InvalidParams(format!("The '{}' parameter was not specified.", param_name)))
        }
    } 
}

// for retrieving info about host systems.
// Note: this is currently designed around Linux
//       conventions, for things like FreeBSD and others
//       it might not match very well...
pub struct SystemDetailsResult {
    // distributor ID - i.e. "Debian"
    pub     distr_id:     String,
    // release number, i.e. "12", or "20.04"
    pub     release:      String,
}

impl SystemDetailsResult {
    pub fn new() -> SystemDetailsResult {
        SystemDetailsResult { distr_id: String::new(), release: String::new() }
    }
}

// generic error enum for action provider methods which return values
pub enum GenericError {
    NotImplemented,
    CommandFailed(String),
    Other(String)
}

pub trait ActionProvider {

    // not sure about this one - ideally it'd be static, but...
    fn name(&self) -> String {
        "".to_string()
    }

    fn get_session_params(&self) -> Option<&ControlSessionParams> {
        None
    }

    // TODO: we might have to make this a derived trait item at some point, but for the moment, we can just
    //       do this...
    fn post_process_command(&self, command: &str) -> String {
        let mut final_command = command.to_string();

        let session_params = self.get_session_params();
        if session_params.is_none() {
            // error.
            eprintln!("Error: ActionProvider does not implement get_session_params()");
            return "".to_string();
        }
        let session_params = session_params.unwrap();
    
        if session_params.user_type == UserType::Sudo {
            final_command.insert_str(0, "sudo ");
        }
    
        if session_params.hide_commands_from_history {
            final_command.insert(0, ' ');
        }
    
        final_command
    }

    fn generic_command(&self, _connection: &mut ControlSession, _action: &ControlAction) -> Result<(), ActionError> {
        Err(ActionError::NotImplemented)
    }

    // this is not really an Action, as it doesn't modify anything, it just returns values, but...
    fn get_system_details(&self, _connection: &mut ControlSession) -> Result<SystemDetailsResult, GenericError> {
        Err(GenericError::NotImplemented)
    }

    fn add_user(&self, _connection: &mut ControlSession, _action: &ControlAction) -> Result<(), ActionError> {
        Err(ActionError::NotImplemented)
    }

    fn create_directory(&self, _connection: &mut ControlSession, _action: &ControlAction) -> Result<(), ActionError> {
        Err(ActionError::NotImplemented)
    }

    fn remove_directory(&self, _connection: &mut ControlSession, _action: &ControlAction) -> Result<(), ActionError> {
        Err(ActionError::NotImplemented)
    }

    fn install_packages(&self, _connection: &mut ControlSession, _action: &ControlAction) -> Result<(), ActionError> {
        Err(ActionError::NotImplemented)
    }

    fn remove_packages(&self, _connection: &mut ControlSession, _action: &ControlAction) -> Result<(), ActionError> {
        Err(ActionError::NotImplemented)
    }

    fn systemctrl(&self, _connection: &mut ControlSession, _action: &ControlAction) -> Result<(), ActionError> {
        Err(ActionError::NotImplemented)
    }

    fn firewall(&self, _connection: &mut ControlSession, _action: &ControlAction) -> Result<(), ActionError> {
        Err(ActionError::NotImplemented)
    }

    fn edit_file(&self, _connection: &mut ControlSession, _action: &ControlAction) -> Result<(), ActionError> {
        Err(ActionError::NotImplemented)
    }

    fn copy_path(&self, _connection: &mut ControlSession, _action: &ControlAction) -> Result<(), ActionError> {
        Err(ActionError::NotImplemented)
    }

    fn remove_file(&self, _connection: &mut ControlSession, _action: &ControlAction) -> Result<(), ActionError> {
        Err(ActionError::NotImplemented)
    }

    fn download_file(&self, _connection: &mut ControlSession, _action: &ControlAction) -> Result<(), ActionError> {
        Err(ActionError::NotImplemented)
    }

    fn transmit_file(&self, _connection: &mut ControlSession, _action: &ControlAction) -> Result<(), ActionError> {
        Err(ActionError::NotImplemented)
    }

    fn receive_file(&self, _connection: &mut ControlSession, _action: &ControlAction) -> Result<(), ActionError> {
        Err(ActionError::NotImplemented)
    }

    fn create_symlink(&self, _connection: &mut ControlSession, _action: &ControlAction) -> Result<(), ActionError> {
        Err(ActionError::NotImplemented)
    }

    fn set_time_zone(&self, _connection: &mut ControlSession, _action: &ControlAction) -> Result<(), ActionError> {
        Err(ActionError::NotImplemented)
    }

    fn disable_swap(&self, _connection: &mut ControlSession, _action: &ControlAction) -> Result<(), ActionError> {
        Err(ActionError::NotImplemented)
    }

    fn create_file(&self, _connection: &mut ControlSession, _action: &ControlAction) -> Result<(), ActionError> {
        Err(ActionError::NotImplemented)
    }

    fn add_group(&self, _connection: &mut ControlSession, _action: &ControlAction) -> Result<(), ActionError> {
        Err(ActionError::NotImplemented)
    }

    fn set_hostname(&self, _connection: &mut ControlSession, _action: &ControlAction) -> Result<(), ActionError> {
        Err(ActionError::NotImplemented)
    }

    fn create_systemd_service(&self, _connection: &mut ControlSession, _action: &ControlAction) -> Result<(), ActionError> {
        Err(ActionError::NotImplemented)
    }
}
