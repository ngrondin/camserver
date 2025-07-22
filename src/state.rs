use std::{sync::{mpsc::Sender, Arc}, time::{SystemTime, UNIX_EPOCH}};

use chrono::{DateTime, Utc};
use log::info;
use rumqttc::{AsyncClient, QoS};
use tokio::sync::Mutex;
use crate::{mqtt::MQTTState, stream::StreamReceiverState};


#[derive(Clone)]
pub struct CameraInfo {
    pub name: String,
    pub ip: String,
    pub stream_id: u8,
    pub lum: u8,
    pub moves: Vec<DateTime<Utc>>,
    pub image: Arc<Vec<u8>>,
    pub last_image: u32,
    pub senders: Vec<Sender<Arc<Vec<u8>>>>
}

impl CameraInfo {
    pub fn new(name: &str, stream_id: u8) -> Self {
        CameraInfo { name: name.to_string(), ip: "".to_string(), stream_id: stream_id,  lum: 0, moves: vec![], image: Arc::new(vec![]), last_image: 0, senders: vec![] }
    }

    pub fn record_movement(&mut self) {
        self.moves.push(SystemTime::now().into());
    }

    pub fn add_sender(&mut self, sender: Sender<Arc<Vec<u8>>>) {
        self.senders.push(sender);
    }
}

pub struct CamerasState {
    cameras: Vec<CameraInfo>
}

impl CamerasState {
    pub fn new() -> Self {
        CamerasState { cameras: vec![] }
    }

    fn add_camera(&mut self, name: &str) -> &mut CameraInfo {
        let new_stream_id = (self.cameras.len() + 1) as u8;
        let new_cam_info = CameraInfo::new(name, new_stream_id);
        self.cameras.push(new_cam_info);
        self.get_mut_camera_from_name(name).unwrap()
    }

    fn get_camera_from_name(&self, name: &str) -> Option<&CameraInfo> {
        let ret = self.cameras.iter().find(|cam_info| cam_info.name == name);
        ret
    }

    fn get_mut_camera_from_name(&mut self, name: &str) -> Option<&mut CameraInfo> {
        let ret = self.cameras.iter_mut().find(|cam_info| cam_info.name == name);
        ret
    }

    fn get_mut_camera_from_stream_id(&mut self, stream_id: u8) -> Option<&mut CameraInfo> {
        let ret = self.cameras.iter_mut().find(|cam_info| cam_info.stream_id == stream_id);
        ret
    }

    fn get_all_cameras(&self) -> Vec<&CameraInfo> {
        let mut ret: Vec<&CameraInfo> = self.cameras.iter().map(|ci| ci).collect();
        ret.sort_by(|a, b| a.name.cmp(&b.name));
        ret
    }

    
}

#[derive(Clone)]
pub struct AppState {
    mqttclient: Arc<Mutex<Option<AsyncClient>>>,
    cameras: Arc<Mutex<CamerasState>>
}

impl AppState {
    pub fn new() -> Self {
        Self { 
            mqttclient: Arc::new(Mutex::new(None)),
            cameras: Arc::new(Mutex::new(CamerasState::new()))
        }
    }

    pub async fn set_camera_stat(&self, name: &str, ip: &str, lum: u8) {
        let mut lock = self.cameras.lock().await;
        //if let Ok(mut lock) = self.cameras.lock().await {
            let cam_info = match lock.get_mut_camera_from_name(name) {
                Some(cam_info) => cam_info,
                None => lock.add_camera(name)
            };
            cam_info.ip = ip.to_string();
            cam_info.lum = lum;
        //};
    }

    pub async fn for_camera<FT, RT>(&self, name: &str, func: FT) -> Option<RT>
    where FT: Fn(&CameraInfo) -> RT {
        let lock = self.cameras.lock().await;
        if let Some(cam_info) = lock.get_camera_from_name(name) {
            let ret = func(cam_info);
            return Some(ret);
        }     
        None
    }

    pub async fn for_mut_camera<FT, RT>(&self, name: &str, func: FT) -> Option<RT>
    where FT: Fn(&mut CameraInfo) -> RT {
        let mut lock = self.cameras.lock().await;
        if let Some(cam_info) = lock.get_mut_camera_from_name(name) {
            let ret = func(cam_info);
            return Some(ret);
        }           
        None
    }

    pub async fn for_all_cameras<FT, RT>(&self, func: FT) -> Vec<RT>
    where FT: Fn(&CameraInfo) -> RT {
        let lock = self.cameras.lock().await;
        let all_cams = lock.get_all_cameras();
        all_cams.iter().map(|cam_info| func(&cam_info)).collect()
    }
}



impl MQTTState for AppState {
    async fn set_mqtt_client(&self, client: AsyncClient) {
        let mut client_option = self.mqttclient.lock().await;
        *client_option = Some(client);
    }

    async fn mqtt_client_subscribe(&self, topic: &str) {
        let client_option = self.mqttclient.lock().await;
        if let Some(client) = client_option.as_ref() {
            let _ = client.subscribe(topic, QoS::AtMostOnce).await;
        }
    }

    async fn mqtt_publish(&self, topic: &str, body: &str) {
        info!("MQTT publish {} {}", topic, body);
        let client_option = self.mqttclient.lock().await;
        if let Some(client) = client_option.as_ref() {
            let _ = client.publish(topic, QoS::AtMostOnce, true, body).await;
        }
    }
}


impl StreamReceiverState for AppState {
    async fn set_stream_image(&self, stream_id: u8, data: Arc<Vec<u8>>) {
        let mut lock = self.cameras.lock().await;
        if let Some(cam) = lock.get_mut_camera_from_stream_id(stream_id) {
            cam.image = data.clone();
            let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().subsec_millis();
            cam.last_image = now;
            for sender in cam.senders.iter() {
                match sender.send(cam.image.clone()) {
                    Err(_) => {},
                    _ => {}
                }
            }
        }
    }
}