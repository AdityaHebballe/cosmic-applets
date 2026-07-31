// Copyright 2026 System76 <info@system76.com>
// SPDX-License-Identifier: GPL-3.0-only

use std::sync::Arc;

use cosmic_settings_audio_client::{self as audio_client, Availability, NodeKind, RouteInfo};
use intmap::IntMap;

pub type DeviceId = u32;
pub type NodeId = u32;

#[derive(Debug, Default)]
pub struct Model {
    pub device_routes: IntMap<DeviceId, Vec<RouteInfo>>,
    pub node_devices: IntMap<NodeId, Option<u32>>,
    pub sinks: Nodes,
    pub sources: Nodes,
    pub streams: Streams,
    pub active_sink: ActiveNode,
    pub active_source: ActiveNode,
    pub default_sink: Option<NodeId>,
    pub default_source: Option<NodeId>,
}

#[derive(Debug, Default)]
pub struct Nodes {
    active: Option<usize>,
    pub sorted_display: Box<[Arc<str>]>,
    pub sorted_index: Box<[u16]>,
    pub balance: Vec<Option<f32>>,
    pub card_profile_device: Vec<Option<u32>>,
    pub description: Vec<String>,
    pub devices: Vec<Option<NodeId>>,
    pub display: Vec<Arc<str>>,
    pub mute: Vec<bool>,
    pub name: Vec<String>,
    pub id: Vec<NodeId>,
    pub volume: Vec<u32>,
}

impl Nodes {
    pub fn active(&self) -> Option<usize> {
        self.active.and_then(|active| {
            self.sorted_index
                .iter()
                .position(|idx| *idx as usize == active)
        })
    }

    pub fn dropdown_sort(&mut self) {
        let mut enumerated_displays = self
            .display
            .clone()
            .into_iter()
            .enumerate()
            .map(|(index, display)| (index as u16, display))
            .collect::<Vec<_>>();
        enumerated_displays.sort_by_key(|v| v.1.clone());
        let (indexes, displays): (Vec<_>, Vec<_>) = enumerated_displays.into_iter().unzip();
        self.sorted_display = displays.into_boxed_slice();
        self.sorted_index = indexes.into_boxed_slice();
    }

    pub fn remove(&mut self, node_id: u32) -> bool {
        let Some(pos) = self.id.iter().position(|id| node_id == *id) else {
            return false;
        };
        self.balance.remove(pos);
        self.card_profile_device.remove(pos);
        self.description.remove(pos);
        self.devices.remove(pos);
        self.display.remove(pos);
        self.mute.remove(pos);
        self.name.remove(pos);
        self.id.remove(pos);
        self.volume.remove(pos);
        self.dropdown_sort();
        if self.active == Some(pos) {
            self.active = None;
        }
        true
    }
}

#[derive(Debug, Default)]
pub struct ActiveNode {
    pub volume_text: String,
    pub volume: u32,
    pub mute: bool,
}

/// Per-application playback/capture streams (sink-input/source-output equivalent).
///
/// Unlike `Nodes`, there is no "active"/"default" stream concept, so this doesn't
/// carry the `sorted_display`/`sorted_index`/`active` machinery `Nodes` needs to back
/// a device-selection dropdown — streams are always enumerated in full.
#[derive(Debug, Default)]
pub struct Streams {
    pub id: Vec<NodeId>,
    pub volume: Vec<u32>,
    pub balance: Vec<Option<f32>>,
    pub mute: Vec<bool>,
    pub display_name: Vec<Arc<str>>,
    pub app_id: Vec<Arc<str>>,
    pub media_name: Vec<Option<Arc<str>>>,
    pub is_input: Vec<bool>,
}

impl Streams {
    pub fn remove(&mut self, node_id: u32) -> bool {
        let Some(pos) = self.id.iter().position(|id| node_id == *id) else {
            return false;
        };
        self.volume.remove(pos);
        self.balance.remove(pos);
        self.mute.remove(pos);
        self.display_name.remove(pos);
        self.app_id.remove(pos);
        self.media_name.remove(pos);
        self.is_input.remove(pos);
        self.id.remove(pos);
        true
    }
}

impl Model {
    pub fn update(&mut self, event: audio_client::Event) {
        tracing::debug!(?event, "update");
        match event {
            audio_client::Event::NodeMute(node_id, mute) => {
                if let Some(pos) = self.sinks.id.iter().position(|id| node_id == *id) {
                    self.sinks.mute[pos] = mute;
                    if self.sinks.active == Some(pos) {
                        self.active_sink.mute = mute;
                    }
                } else if let Some(pos) = self.sources.id.iter().position(|id| node_id == *id) {
                    self.sources.mute[pos] = mute;
                    if self.sources.active == Some(pos) {
                        self.active_source.mute = mute;
                    }
                } else if let Some(pos) = self.streams.id.iter().position(|id| node_id == *id) {
                    self.streams.mute[pos] = mute;
                }
            }

            audio_client::Event::NodeVolume(node_id, volume, balance) => {
                if let Some(pos) = self.sinks.id.iter().position(|id| node_id == *id) {
                    self.sinks.volume[pos] = volume;
                    self.sinks.balance[pos] = balance;
                    if self.default_sink.as_ref().is_some_and(|&id| id == node_id)
                        && let Some(pos) = self.sinks.active
                    {
                        self.active_sink.mute = self.sinks.mute[pos];
                        self.active_sink.volume = self.sinks.volume[pos];
                        self.active_sink.volume_text = self.active_sink.volume.to_string();
                    }
                } else if let Some(pos) = self.sources.id.iter().position(|id| node_id == *id) {
                    self.sources.volume[pos] = volume;
                    self.sources.balance[pos] = balance;
                    if self
                        .default_source
                        .as_ref()
                        .is_some_and(|&id| id == node_id)
                        && let Some(pos) = self.sources.active
                    {
                        self.active_source.mute = self.sources.mute[pos];
                        self.active_source.volume = self.sources.volume[pos];
                        self.active_source.volume_text = self.active_source.volume.to_string();
                    }
                } else if let Some(pos) = self.streams.id.iter().position(|id| node_id == *id) {
                    self.streams.volume[pos] = volume;
                    self.streams.balance[pos] = balance;
                }
            }

            audio_client::Event::DefaultSink(node_id) => {
                self.default_sink = Some(node_id);
                if let Some(pos) = self.sinks.id.iter().position(|&id| id == node_id) {
                    self.sinks.active = Some(pos);
                    self.active_sink.mute = self.sinks.mute[pos];
                    self.active_sink.volume = self.sinks.volume[pos];
                    self.active_sink.volume_text = self.active_sink.volume.to_string();
                }
            }

            audio_client::Event::DefaultSource(node_id) => {
                self.default_source = Some(node_id);
                if let Some(pos) = self.sources.id.iter().position(|&id| id == node_id) {
                    self.sources.active = Some(pos);
                    self.active_source.mute = self.sources.mute[pos];
                    self.active_source.volume = self.sources.volume[pos];
                    self.active_source.volume_text = self.active_source.volume.to_string();
                }
            }

            audio_client::Event::Node(node_id, node) => {
                self.node_devices.insert(node_id, node.device_id);
                if matches!(node.kind, NodeKind::StreamOutput | NodeKind::StreamInput) {
                    let app_id: Arc<str> = node
                        .application_binary
                        .clone()
                        .or_else(|| node.application_name.clone())
                        .unwrap_or_else(|| node.name.clone())
                        .into();
                    let display_name: Arc<str> = node
                        .application_name
                        .clone()
                        .or_else(|| node.application_binary.clone())
                        .unwrap_or_else(|| node.name.clone())
                        .into();
                    let media_name: Option<Arc<str>> =
                        node.media_name.clone().map(Into::into);
                    let is_input = matches!(node.kind, NodeKind::StreamInput);

                    if let Some(pos) = self.streams.id.iter().position(|&id| id == node_id) {
                        self.streams.app_id[pos] = app_id;
                        self.streams.display_name[pos] = display_name;
                        self.streams.media_name[pos] = media_name;
                        self.streams.is_input[pos] = is_input;
                    } else {
                        self.streams.id.push(node_id);
                        self.streams.volume.push(0);
                        self.streams.balance.push(None);
                        self.streams.mute.push(false);
                        self.streams.display_name.push(display_name);
                        self.streams.app_id.push(app_id);
                        self.streams.media_name.push(media_name);
                        self.streams.is_input.push(is_input);
                    }

                    return;
                }

                if matches!(node.kind, NodeKind::Sink) {
                    let pos = if let Some(pos) = self.sinks.id.iter().position(|&id| id == node_id)
                    {
                        self.sinks.description[pos] = self.translate(&node.description);
                        self.sinks.name[pos] = node.name;
                        self.sinks.card_profile_device[pos] = node.card_profile_device;
                        pos
                    } else {
                        self.sinks.display.push(Arc::default());
                        self.sinks
                            .description
                            .push(self.translate(&node.description));
                        self.sinks.id.push(node_id);
                        self.sinks.volume.push(0);
                        self.sinks.balance.push(None);
                        self.sinks.mute.push(false);
                        self.sinks.name.push(node.name);
                        self.sinks.devices.push(node.device_id);
                        self.sinks
                            .card_profile_device
                            .push(node.card_profile_device);
                        self.sinks.id.len() - 1
                    };

                    self.sinks.display[pos] = node
                        .device_id
                        .zip(node.card_profile_device)
                        .and_then(|(device_id, node_card_profile_device)| {
                            let routes = self.device_routes.get(device_id)?;
                            for route in routes {
                                if matches!(route.availability, Availability::No) || !route.is_sink
                                {
                                    continue;
                                }

                                if route.devices.contains(&node_card_profile_device) {
                                    return Some(node_name(
                                        &self.translate(&route.description),
                                        &self.sinks.description[pos],
                                    ));
                                }
                            }

                            None
                        })
                        .unwrap_or_else(|| {
                            node_name(
                                &node.device_profile_description,
                                &self.sinks.description[pos],
                            )
                        });

                    self.sinks.dropdown_sort();

                    if let Some(default_node_id) = self.default_sink
                        && default_node_id == node_id
                    {
                        self.sinks.active = Some(pos);
                        self.active_sink.mute = self.sinks.mute[pos];
                        self.active_sink.volume = self.sinks.volume[pos];
                        self.active_sink.volume_text = self.active_sink.volume.to_string();
                    }
                } else {
                    let pos =
                        if let Some(pos) = self.sources.id.iter().position(|&id| id == node_id) {
                            self.sources.description[pos] = self.translate(&node.description);
                            self.sources.name[pos] = node.name;
                            self.sources.card_profile_device[pos] = node.card_profile_device;
                            pos
                        } else {
                            self.sources
                                .description
                                .push(self.translate(&node.description));
                            self.sources.display.push(Arc::default());
                            self.sources.id.push(node_id);
                            self.sources.volume.push(0);
                            self.sources.balance.push(None);
                            self.sources.mute.push(false);
                            self.sources.name.push(node.name);
                            self.sources.devices.push(node.device_id);
                            self.sources
                                .card_profile_device
                                .push(node.card_profile_device);
                            self.sources.id.len() - 1
                        };

                    if let Some(name) = node
                        .device_id
                        .zip(node.card_profile_device)
                        .map(|(device_id, node_card_profile_device)| {
                            let routes = self.device_routes.get(device_id)?;
                            for route in routes {
                                if route.is_sink || matches!(route.availability, Availability::No) {
                                    continue;
                                }

                                if route.devices.contains(&node_card_profile_device) {
                                    return Some(node_name(
                                        &self.translate(&route.description),
                                        &self.sources.description[pos],
                                    ));
                                }
                            }

                            None
                        })
                        .unwrap_or_else(|| {
                            Some(node_name(
                                &node.device_profile_description,
                                &self.sources.description[pos],
                            ))
                        })
                    {
                        self.sources.display[pos] = name;
                        self.sources.dropdown_sort();
                    } else {
                        // Remove sources that are unplugged.
                        self.sources.remove(node_id);
                        return;
                    }

                    if let Some(default_node_id) = self.default_source
                        && default_node_id == node_id
                    {
                        self.sources.active = Some(pos);
                        self.active_source.mute = self.sources.mute[pos];
                        self.active_source.volume = self.sources.volume[pos];
                        self.active_source.volume_text = self.active_source.volume.to_string();
                    }
                }
            }

            audio_client::Event::ActiveRoute(device_id, _index, route) => {
                self.update_device_names(device_id, &route);
            }

            audio_client::Event::Route(device_id, index, route) => {
                let routes = self.device_routes.entry(device_id).or_default();
                if index == 0 {
                    *routes = vec![route];
                } else {
                    if routes.len() < index as usize + 1 {
                        let additional = (index as usize + 1) - routes.capacity();
                        routes.reserve_exact(additional);
                        routes.extend(std::iter::repeat_n(RouteInfo::default(), additional));
                    }
                    routes[index as usize] = route;
                }
            }

            audio_client::Event::RemoveNode(node_id) => {
                self.node_devices.remove(node_id);

                if !self.sinks.remove(node_id) && !self.sources.remove(node_id) {
                    self.streams.remove(node_id);
                }
            }

            audio_client::Event::RemoveDevice(device_id) => {
                self.device_routes.remove(device_id);
            }

            _ => (),
        }
    }

    fn update_device_names(&mut self, device_id: DeviceId, route: &RouteInfo) {
        if matches!(route.availability, Availability::No) {
            return;
        }

        let compatible_nodes = self.node_devices.iter().filter_map(|(node, &dev_id)| {
            if dev_id? == device_id {
                Some(node)
            } else {
                None
            }
        });

        if route.is_sink {
            for n_id in compatible_nodes {
                let Some(pos) = self.sinks.id.iter().position(|&node| node == n_id) else {
                    continue;
                };

                let Some(card_profile_device) = self.sinks.card_profile_device[pos] else {
                    continue;
                };

                if route.devices.contains(&card_profile_device) {
                    self.sinks.display[pos] = node_name(
                        &self.translate(&route.description),
                        &self.sinks.description[pos],
                    );
                    self.sinks.dropdown_sort();
                    break;
                }
            }
        } else {
            for n_id in compatible_nodes {
                let Some(pos) = self.sources.id.iter().position(|&node| node == n_id) else {
                    continue;
                };

                let Some(card_profile_device) = self.sources.card_profile_device[pos] else {
                    continue;
                };

                if route.devices.contains(&card_profile_device) {
                    self.sources.display[pos] = node_name(
                        &self.translate(&route.description),
                        &self.sources.description[pos],
                    );
                    self.sources.dropdown_sort();
                    break;
                }
            }
        }
    }

    pub fn translate(&self, description: &str) -> String {
        description
            .replace("High Definition", "HD")
            .replace("DisplayPort", "DP")
            .replace("Controller", "")
    }
}

fn node_name(route: &str, node: &str) -> Arc<str> {
    if route.is_empty() {
        node.to_owned()
    } else {
        [route, " - ", node].concat()
    }
    .into()
}
