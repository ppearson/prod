/*
 Prod
 Copyright 2021-2022 Peter Pearson.
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

use super::common_actions_linux;
use super::common_actions_unix;

use super::control_actions::{ActionProvider, ActionResult, ControlAction};
use super::control_common::{ControlSession, ControlSessionParams};

pub struct AProviderLinuxFedora {
    // params which give us some hints as to context of session, i.e. username - sudo vs root, etc.
    session_params: ControlSessionParams,
}

impl AProviderLinuxFedora {
    pub fn new(session_params: ControlSessionParams) -> AProviderLinuxFedora {
        AProviderLinuxFedora { session_params }
    }

    pub fn name() -> String {
        return "linux_fedora".to_string();
    }
}

impl ActionProvider for AProviderLinuxFedora {
    fn name(&self) -> String {
        return "linux_fedora".to_string();
    }

    fn get_session_params(&self) -> Option<&ControlSessionParams> {
        return Some(&self.session_params);
    }

    fn generic_command(&self, connection: &mut ControlSession, action: &ControlAction) -> ActionResult {
        return common_actions_unix::generic_command(self, connection, action);
    }

    fn add_user(&self, connection: &mut ControlSession, action: &ControlAction) -> ActionResult {
        return common_actions_linux::add_user(self, connection, action);
    }

    fn create_directory(&self, connection: &mut ControlSession, action: &ControlAction) -> ActionResult {
        return common_actions_unix::create_directory(self, connection, action);
    }

    fn install_packages(&self, connection: &mut ControlSession, action: &ControlAction) -> ActionResult {
        let packages_string;
        if let Some(package) = action.params.get_string_value("package") {
            // single package for convenience...
            packages_string = package;
        }
        else if action.params.has_value("packages") {
            let packages = action.params.get_values_as_vec_of_strings("packages");
            packages_string = packages.join(" ");
        }
        else {
            return ActionResult::InvalidParams("No 'package' string parameter or 'packages' string array parameter were specified.".to_string());
        }

        if packages_string.is_empty() {
            return ActionResult::InvalidParams("The resulting 'packages' string list was empty.".to_string());
        }

        // by default, update the list of packages, as with some providers,
        // this needs to be done first, otherwise packages can't be found...
        let update_packages = action.params.get_value_as_bool("update", true);
        if update_packages {
            let dnf_command = "dnf -y update".to_string();
            connection.conn.send_command(&self.post_process_command(&dnf_command));
        }

        let dnf_command = format!("dnf -y install {}", packages_string);
        connection.conn.send_command(&self.post_process_command(&dnf_command));

        if let Some(str) = connection.conn.get_previous_stderr_response() {
            println!("installPackages error: {}", str);
            return ActionResult::Failed(str.to_string());
        }

        return ActionResult::Success;
    }

    fn remove_packages(&self, connection: &mut ControlSession, action: &ControlAction) -> ActionResult {
        let packages_string;
        if let Some(package) = action.params.get_string_value("package") {
            // single package for convenience...
            packages_string = package;
        }
        else if action.params.has_value("packages") {
            let packages = action.params.get_values_as_vec_of_strings("packages");
            packages_string = packages.join(" ");
        }
        else {
            return ActionResult::InvalidParams("No 'package' string parameter or 'packages' string array parameter were specified.".to_string());
        }

        if packages_string.is_empty() {
            return ActionResult::InvalidParams("The resulting 'packages' string list was empty.".to_string());
        }

        let dnf_command = format!("dnf -y remove {}", packages_string);
        connection.conn.send_command(&self.post_process_command(&dnf_command));

        if let Some(str) = connection.conn.get_previous_stderr_response() {
            println!("removePackages error: {}", str);
            return ActionResult::Failed(str.to_string());
        }

        return ActionResult::Success;
    }

    fn systemctrl(&self, connection: &mut ControlSession, action: &ControlAction) -> ActionResult {
        return common_actions_linux::systemctrl(self, connection, action);
    }

    fn firewall(&self, connection: &mut ControlSession, action: &ControlAction) -> ActionResult {
        // fedora apparently needs ufw firewall enabled first before adding rules, despite the
        // man page saying it's supported, and it working on debian/ubuntu
        return common_actions_linux::firewall(self, connection, action, true);
    }

    fn edit_file(&self, connection: &mut ControlSession, action: &ControlAction) -> ActionResult {
        return common_actions_unix::edit_file(self, connection, action);
    }

    fn copy_path(&self, connection: &mut ControlSession, action: &ControlAction) -> ActionResult {
        return common_actions_unix::copy_path(self, connection, action);
    }

    fn download_file(&self, connection: &mut ControlSession, action: &ControlAction) -> ActionResult {
        return common_actions_unix::download_file(self, connection, action);
    }

    fn transmit_file(&self, connection: &mut ControlSession, action: &ControlAction) -> ActionResult {
        return common_actions_unix::transmit_file(self, connection, action);
    }

    fn receive_file(&self, connection: &mut ControlSession, action: &ControlAction) -> ActionResult {
        return common_actions_unix::receive_file(self, connection, action);
    }

    fn create_symlink(&self, connection: &mut ControlSession, action: &ControlAction) -> ActionResult {
        return common_actions_unix::create_symlink(self, connection, action);
    }

    fn set_time_zone(&self, connection: &mut ControlSession, action: &ControlAction) -> ActionResult {
        return common_actions_linux::set_time_zone(self, connection, action);
    }
}
