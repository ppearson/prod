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

pub struct AProviderLinuxDebian {
    // params which give us some hints as to context of session, i.e. username - sudo vs root, etc.
    session_params: ControlSessionParams,
}

impl AProviderLinuxDebian {
    pub fn new(session_params: ControlSessionParams) -> AProviderLinuxDebian {
        AProviderLinuxDebian { session_params }
    }

    pub fn name() -> String {
        return "linux_debian".to_string();
    }
}

impl ActionProvider for AProviderLinuxDebian {
    fn name(&self) -> String {
        return "linux_debian".to_string();
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
        // use apt-get, because the commands for that will apparently be much more stable, compared to apt
        // which might change as it's designed to be more user-facing...

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

        // with some providers (Vultr), apt-get runs automatically just after the instance first starts,
        // so we can't run apt-get manually, as the lock file is locked, so wait until apt-get has stopped running
        // by default... 
        let wait_for_apt_get_lockfile = action.params.get_value_as_bool("waitForPMToFinish", true);
        if wait_for_apt_get_lockfile {
            let mut try_count = 0;
            while try_count < 20 {
                connection.conn.send_command(&self.post_process_command("pidof apt-get"));

                if !connection.conn.had_command_response() {
                    // it's likely no longer running, so we can continue...
                    break;
                }

                // TODO: only print this once eventually, but might be useful like this for the moment...
                println!("Waiting for existing apt-get to finish before installing packages...");

                // sleep a bit to give things a chance...
                std::thread::sleep(std::time::Duration::from_secs(20));

                try_count += 1;
            }
        }

        // unattended-upgr

        // TODO: might be worth polling for locks on /var/lib/dpkg/lock-frontend ?

        // by default, update the list of packages, as with some providers,
        // this needs to be done first, otherwise packages can't be found...
        let update_packages = action.params.get_value_as_bool("update", true);
        if update_packages {
            let apt_get_command = "apt-get -y update".to_string();
            connection.conn.send_command(&self.post_process_command(&apt_get_command));
        }

        // Note: first time around, unless we export this DEBIAN_FRONTEND env variable, we get a
        //       debconf / dpkg-preconfigure issue with $TERM apparently being unset, and complaints
        //       about the frontend not being useable. Interestingly, without setting the env variable,
        //       trying again after the first failure works, and it's not time-dependent...

        let apt_get_command = format!("export DEBIAN_FRONTEND=noninteractive; apt-get -y install {}", packages_string);
        connection.conn.send_command(&self.post_process_command(&apt_get_command));

        if let Some(str) = connection.conn.get_previous_stderr_response() {
            println!("installPackages error: {}", str);
            return ActionResult::Failed(str.to_string());
        }

        return ActionResult::Success;
    }

    fn remove_packages(&self, connection: &mut ControlSession, action: &ControlAction) -> ActionResult {
        // use apt-get, because the commands for that will apparently be much more stable, compared to apt
        // which might change as it's designed to be more user-facing...

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

        // with some providers (Vultr), apt-get runs automatically just after the instance first starts,
        // so we can't run apt-get manually, as the lock file is locked, so wait until apt-get has stopped running
        // by default... 
        let wait_for_apt_get_lockfile = action.params.get_value_as_bool("waitForPMToFinish", true);
        if wait_for_apt_get_lockfile {
            let mut try_count = 0;
            while try_count < 20 {
                connection.conn.send_command(&self.post_process_command("pidof apt-get"));

                if !connection.conn.had_command_response() {
                    // it's likely no longer running, so we can continue...
                    break;
                }

                // TODO: only print this once eventually, but might be useful like this for the moment...
                println!("Waiting for existing apt-get to finish before removing packages...");

                // sleep a bit to give things a chance...
                std::thread::sleep(std::time::Duration::from_secs(20));

                try_count += 1;
            }
        }

        let apt_get_command = format!("export DEBIAN_FRONTEND=noninteractive; apt-get -y remove {}", packages_string);
        connection.conn.send_command(&self.post_process_command(&apt_get_command));

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
        // debian doesn't need ufw firewall enabled first before adding rules
        return common_actions_linux::firewall(self, connection, action, false);
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

