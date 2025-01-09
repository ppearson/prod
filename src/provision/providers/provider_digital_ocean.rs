/*
 Prod
 Copyright 2021-2025 Peter Pearson.
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

use ureq;
use ureq::Error;
use serde::{Deserialize, Serialize};

use std::collections::BTreeSet;

use crate::provision::provision_provider::ProvisionProvider;
use crate::provision::provision_common::{ProvisionActionType, ProvisionActionResult, ActionResultValues, ProvisionResponseWaitType};
use crate::provision::provision_manager::ListType;
use crate::provision::provision_params::ProvisionParams;

use crate::column_list_printer::{ColumnListPrinter, Alignment};

#[derive(Serialize, Deserialize)]
struct SizeResultItem {
    slug: String,
    memory: u32,
    vcpus: u32,
    disk: u32,
    transfer: f32,
    description: String,

    price_monthly: f32,
    price_hourly: f32,
}

#[derive(Serialize, Deserialize)]
struct SizeListResults {
    sizes: Vec<SizeResultItem>
}

#[derive(Serialize, Deserialize)]
struct ImageResultItem {
    id: u64,
    name: String,
    distribution: String,
    slug: String,
    public: bool,
    min_disk_size: u32,

    #[serde(rename = "type")]
    ttype: String,

    size_gigabytes: f32,
    description: String,
    status: String,
}

#[derive(Serialize, Deserialize)]
struct ImageListResults {
    images: Vec<ImageResultItem>
}

#[derive(Serialize, Deserialize)]
struct RegionResultItem {
    name: String,
    slug: String,
    available: bool,
}

#[derive(Serialize, Deserialize)]
struct RegionListResults {
    regions: Vec<RegionResultItem>
}

#[derive(Clone, Debug)]
#[derive(Serialize, Deserialize)]
struct Networkv4 {
    ip_address:     String,
    netmask:        String,
    gateway:        String,

    #[serde(rename = "type")]
    ttype:          String,    
}

#[derive(Clone, Debug)]
#[derive(Serialize, Deserialize)]
struct DropletNetworks {
    v4:         Vec<Networkv4>,
}

#[derive(Clone, Debug)]
#[derive(Serialize, Deserialize)]
struct DropletInstanceDetailsInner {
    id:         u64,
    name:       String,
    memory:     u32,
    disk:       u32,

    locked:     bool,

    status:     String,

    networks:   DropletNetworks,
}

#[derive(Serialize, Deserialize)]
struct DropletInstanceDetails {
    droplet: DropletInstanceDetailsInner,
}

impl DropletInstanceDetails {
    fn get_public_v4_network_ip(&self) -> Option<String> {
        for net in &self.droplet.networks.v4 {
            if net.ttype == "public" && !net.ip_address.is_empty() {
                return Some(net.ip_address.clone());
            }
        }

        None
    }
}

pub struct ProviderDigitalOcean {
    digital_ocean_api_token: String,
}

impl ProviderDigitalOcean {
    pub fn new() -> ProviderDigitalOcean {
        ProviderDigitalOcean { digital_ocean_api_token: String::new() }
    }
}

impl ProvisionProvider for ProviderDigitalOcean {
    fn name(&self) -> String {
        "digital_ocean".to_string()
    }

    fn supports_interactive(&self) -> bool {
        true
    }

    fn prompt_interactive(&self) -> Vec<(String, String)> {
        let mut items = Vec::new();
        items.push(("API_TOKEN".to_string(), "API Token to use Digital Ocean API".to_string()));
        items
    }

    fn configure_interactive(&mut self) -> bool {
        false
    }

    fn configure(&mut self) -> bool {
        let digital_ocean_api_token_env = std::env::var("PROD_DIGITAL_OCEAN_API_TOKEN");
        match digital_ocean_api_token_env {
            Err(_e) => {
                // silently fail...
//                eprintln!("Error: $PROD_DIGITAL_OCEAN_API_TOKEN not set correctly.");
                return false;
            }
            Ok(v) => {
                self.digital_ocean_api_token = v.trim().to_string();
                return true;
            }
        }
    }

    fn is_configured(&self) -> bool {
        !self.digital_ocean_api_token.is_empty()
    }

    // actual commands

    fn list_available(&self, list_type: ListType) -> bool {
        let url = match list_type {
            ListType::Plans => "sizes",
            ListType::Regions => "regions",
            ListType::OSs => "images",
            _ => {
                return false;
            }
        };

        // Note: DigitalOcean requires an API token header even for GET requests
        //       to list things, which is a bit annoying...
        if self.digital_ocean_api_token.is_empty() {
            eprintln!("Digital Ocean requires an API token to be used for list API requests. Please set $PROD_DIGITAL_OCEAN_API_TOKEN.");
            return false;
        }

        let resp = ureq::get(&format!("https://api.digitalocean.com/v2/{}", url))
            .set("Authorization", &format!("Bearer {}", self.digital_ocean_api_token))
            .call();
        
        if resp.is_err() {
            eprintln!("Error querying api.digitalocean.com for list request: {:?}", resp.err());
            return false;
        }

        let resp_string = resp.unwrap().into_string().unwrap();

        if list_type == ListType::Regions {
            let results: RegionListResults = serde_json::from_str(&resp_string).unwrap();

            println!("{} regions:", results.regions.len());

            let mut clp = ColumnListPrinter::new(3)
                .add_titles(["ID", "Name", "Available"]);

            for region in &results.regions {
                clp.add_row_strings(&[&region.slug, &region.name, if region.available {"true"} else {"false"}]);
            }

            print!("{}", clp);
        }
        else if list_type == ListType::Plans {
            let results: SizeListResults = serde_json::from_str(&resp_string).unwrap();

            println!("{} plans:", results.sizes.len());

            let mut clp = ColumnListPrinter::new(7)
                .set_alignment_multiple(&[2usize, 3, 4, 5, 6], Alignment::Right)
                .add_titles(["ID", "Desc", "vcpus", "Memory", "Disk", "Transfer", "Price"]);

            for size in &results.sizes {
                clp.add_row_strings(&[&size.slug, &size.description, &format!("{}", size.vcpus), &format!("{} MB", size.memory),
                                         &format!("{} GB", size.disk), &format!("{} TB", size.transfer), &format!("${}", size.price_monthly)]);
            }

            print!("{}", clp);
        }
        else if list_type == ListType::OSs {
            let results: ImageListResults = serde_json::from_str(&resp_string).unwrap();

            println!("{} OS images:", results.images.len());

            let mut clp = ColumnListPrinter::new(4)
                .add_titles(["ID", "Distribution", "Description", "Status"]);

            for image in &results.images {
                clp.add_row_strings(&[&format!("{}", image.id), &image.distribution, &image.description, &image.status]);
            }

            print!("{}", clp);
        }
        else {
            println!("{}", resp_string);
        }
        
        true
    }

    fn get_required_params_for_action(&self, action: ProvisionActionType) -> BTreeSet<&str> {
        let mut params = BTreeSet::new();
        if action == ProvisionActionType::CreateInstance {
            params.insert("name");
            params.insert("region");
            params.insert("size");
            params.insert("image");
        }
        params
    }

    fn create_instance(&self, params: &ProvisionParams, _dry_run: bool) -> ProvisionActionResult {
        let name_str = params.get_string_value("name", "");
        let region_str = params.get_string_value("region", "");
        let size_str = params.get_string_value("size", "");
        let image_str = params.get_string_value("image", "");
        let ipv6 = params.get_string_value_as_bool("ipv6", false);
        let backups = params.get_string_value_as_bool("backups", false);
        let monitoring = params.get_string_value_as_bool("monitoring", false);

        // Note: get_string_array() will return even single strings as an array by-design...
        let ssh_keys = params.get_string_array("ssh_keys");

        let mut json_value = ureq::json!({
            "name": name_str,
            "region": region_str,
            "size": size_str,
            "image": image_str,
            "ipv6": ipv6,
            "backups": backups,
            "monitoring": monitoring,
        });

        if let Some(ssh_keys_array) = ssh_keys {
            json_value.as_object_mut().unwrap().insert("ssh_keys".to_string(), serde_json::to_value(ssh_keys_array).unwrap());
        }

        let resp = ureq::post("https://api.digitalocean.com/v2/droplets")
            .set("Authorization", &format!("Bearer {}", self.digital_ocean_api_token))
            .send_json(json_value);

        // TODO: there's an insane amount of boilerplate error handling and response
        //       decoding going on here... Try and condense it...
        
        // TODO: make some of this re-useable for multiple actions...
        if resp.is_err() {
            match resp.err() {
                Some(Error::Status(code, response)) => {
                    // server returned an error code we weren't expecting...
                    match code {
                        401 => {
                            eprintln!("Error: authentication error with Digital Ocean API: {}", response.into_string().unwrap());
                            return ProvisionActionResult::ErrorAuthenticationIssue("".to_string());
                        },
                        404 => {
                            eprintln!("Error: Not found response from Digital Ocean API: {}", response.into_string().unwrap());
                            return ProvisionActionResult::Failed("".to_string());
                        }
                        _ => {
                            
                        }
                    }
                    eprintln!("Error creating instance0: code: {}, resp: {:?}", code, response);
                },
                Some(e) => {
                    eprintln!("Error creating instance1: {:?}", e);
                }
                _ => {
                    // some sort of transport/io error...
                    eprintln!("Error creating instance2: ");
                }
            }
            return ProvisionActionResult::Failed("".to_string());
        }
        
        let resp_string = resp.unwrap().into_string().unwrap();

        let droplet_details = serde_json::from_str(&resp_string);
        if droplet_details.is_err() {
            eprintln!("Error parsing json response from api.digitalocean.com: {}", resp_string);
            return ProvisionActionResult::Failed("".to_string());
        }

        let droplet_details: DropletInstanceDetails = droplet_details.unwrap();
        let droplet_details = droplet_details.droplet;

        let mut result_values = ActionResultValues::new();

        result_values.values.insert("id".to_string(), droplet_details.id.to_string());

        eprintln!("Digital Ocean droplet created with id: {} ...", droplet_details.id);

        if params.wait_type == ProvisionResponseWaitType::ReturnImmediatelyAfterAPIRequest {
            return ProvisionActionResult::ActionCreatedInProgress(result_values);
        }

        eprintln!("Waiting for droplet instance to spool up with IP address...");

        // to get hold of the IP address, we need to do an additional API query to the
        // get droplet API as it's still in the process of being spooled up..

        let mut found_ip = false;

        let max_tries = 10;
        let mut try_count = 0;

        let droplet_id = result_values.values.get("id").unwrap().clone();

        while try_count < max_tries {
            // sleep a bit to give things a chance...
            std::thread::sleep(std::time::Duration::from_secs(15));

            let droplet_details = self.get_droplet_details(&droplet_id);
            if droplet_details.is_err() {
                return droplet_details.err().unwrap();
            }
            let droplet_details = droplet_details.unwrap();

            let ipv4_address = droplet_details.get_public_v4_network_ip();
            if !found_ip {
                if let Some(str) = ipv4_address {
                    found_ip = true;
                    result_values.values.insert("ip".to_string(), str.clone());

                    eprintln!("Have droplet instance IP: {}", str.clone());

                    // so we now have an IP, but the droplet still isn't ready to be used, but maybe that's
                    // all we need...
                    if params.wait_type == ProvisionResponseWaitType::WaitForResourceCreationOrModification {
                        // this is sufficient, so return out...
                        return ProvisionActionResult::ActionCreatedInProgress(result_values);
                    }

                    eprintln!("Waiting for server to finish install/setup...");
                }
            }

            if found_ip && droplet_details.droplet.status == "active" {
                return ProvisionActionResult::ActionCreatedDone(result_values);
            }

            try_count += 1;
        }
        
        ProvisionActionResult::ActionCreatedInProgress(result_values)
    }

    fn delete_instance(&self, params: &ProvisionParams, _dry_run: bool) -> ProvisionActionResult {
        let instance_id = params.get_string_value("instance_id", "");
        let full_url = format!("https://api.digitalocean.com/v2/droplets/{}", instance_id);

        let resp = ureq::delete(&full_url)
            .set("Authorization", &format!("Bearer {}", self.digital_ocean_api_token))
            .call();

        // TODO: there's an insane amount of boilerplate error handling and response
        //       decoding going on here... Try and condense it...
        
        // TODO: make some of this re-useable for multiple actions...
        if resp.is_err() {
            match resp.err() {
                Some(Error::Status(code, response)) => {
                    // server returned an error code we weren't expecting...
                    match code {
                        400 => {
                            eprintln!("Error: Bad request error: {}", response.into_string().unwrap());
                            return ProvisionActionResult::ErrorAuthenticationIssue("".to_string());
                        },
                        401 => {
                            eprintln!("Error: authentication error with Digital Ocean API: {}", response.into_string().unwrap());
                            return ProvisionActionResult::ErrorAuthenticationIssue("".to_string());
                        },
                        404 => {
                            eprintln!("Error: Not found response from Digital Ocean API: {}", response.into_string().unwrap());
                            return ProvisionActionResult::Failed("".to_string());
                        }
                        _ => {
                            
                        }
                    }
                    eprintln!("Error deleting droplet instance0: code: {}, resp: {:?}", code, response);
                },
                Some(e) => {
                    eprintln!("Error deleting droplet instance1: {:?}", e);
                }
                _ => {
                    // some sort of transport/io error...
                    eprintln!("Error deleting instance2: ");
                }
            }
            return ProvisionActionResult::Failed("".to_string());
        }

        // TODO: should be response code 204 for success...
        
        // response should be empty...
        let _resp_string = resp.unwrap().into_string().unwrap();

        ProvisionActionResult::ActionCreatedInProgress(ActionResultValues::new())
    }
}

impl ProviderDigitalOcean {
    fn get_droplet_details(&self, droplet_id: &str) -> Result<DropletInstanceDetails, ProvisionActionResult> {
        let url = format!("https://api.digitalocean.com/v2/droplets/{}", &droplet_id);
        let get_droplet_response = ureq::get(&url)
            .set("Authorization", &format!("Bearer {}", self.digital_ocean_api_token))
            .call();
        
        if let Err(error) = get_droplet_response {
            let resp_string = error.to_string();
            eprintln!("Error getting json response from digitalocean.com for get droplet call: {}", resp_string);
            return Err(ProvisionActionResult::Failed("".to_string()));
        }

        let resp_string = get_droplet_response.unwrap().into_string().unwrap();

        let droplet_details = serde_json::from_str(&resp_string);
        if droplet_details.is_err() {
            eprintln!("Error parsing json response from digitalocean.com for get droplet call: {}", resp_string);
            return Err(ProvisionActionResult::Failed("".to_string()));
        }
        let droplet_details: DropletInstanceDetails = droplet_details.unwrap();

        Ok(droplet_details)
    }
}
