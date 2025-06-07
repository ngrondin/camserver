use std::{env, future::Future, sync::{Arc, Mutex}, time::Duration};

use regex::Regex;
use rumqttc::{AsyncClient, Event::{Incoming, Outgoing}, EventLoop, MqttOptions, Packet::{Publish, Connect, Disconnect}};
use serde_json::Value;
use tokio::task;

use crate::mqtt;

type HandlerFn0<T> = fn(&T, &str, &Value);

pub trait MQTTState {
    fn set_mqtt_client(&self, client: AsyncClient);
    //fn mqtt_client_subscribe(&self, topic: &str) -> impl Future<Output=()> + Send;
    async fn mqtt_client_subscribe(&self, topic: &str);
    async fn mqtt_publish(&self, topic: &str, body: &str);
}

pub struct Subscription<T: MQTTState + Clone + Send> {
    topic: String,
    regex: Regex,
    func: HandlerFn0<T>
}

#[derive(Clone)]
pub struct MQTTServer<T: MQTTState + Clone + Send> {
    state: Arc<T>,
    subs: Arc<Mutex<Vec<Subscription<T>>>>
}

impl<T: MQTTState + 'static + Sync + Send + Clone> MQTTServer<T> {
    pub fn new(state: T) -> Self {
        let mqtt_ip = env::var("MQTT_BROKER").unwrap();
        let mut mqttoptions = MqttOptions::new("camserver", &mqtt_ip, 1883);
        mqttoptions.set_keep_alive(Duration::from_secs(5));
        let (client, eventloop) = AsyncClient::new(mqttoptions, 10);
        state.set_mqtt_client(client);
        let arcstate = Arc::new(state);
        let arcsubs = Arc::new(Mutex::new(Vec::new()));
        let ret = MQTTServer {
            state: arcstate.clone(),
            subs: arcsubs.clone()
        };
        spawn_mqtt_thread(eventloop, Arc::new(ret.clone()));
        ret
    }

    pub async fn subscribe(&self, topic: &str, f: HandlerFn0<T>) {
        let mut subs_locked = self.subs.lock().unwrap();
        let retext = topic
            .replace("/", "\\/")
            .replace("+", "[a-zA-Z0-9_]*")
            .replace("#", "[a-zA-Z0-9_\\/]*");
        let regex = Regex::new(retext.as_str()).unwrap();
        let subs = Subscription {topic: topic.to_string(), regex, func: f};
        subs_locked.push(subs);
        self.state.mqtt_client_subscribe(topic).await;
    }

    pub async fn resubscribe(&self) {
        let topics: Vec<String> = self.subs.lock().unwrap().iter().map(|s| {s.topic.to_string()}).collect();
        for topic in topics {
            self.state.mqtt_client_subscribe(&topic).await;
        }
    }

    pub async fn receive(&self, topic: &str, body: &str) {
        let json_value: Value = serde_json::from_str(body).unwrap();
        let subs = self.subs.lock().unwrap();
        for sub in subs.iter() {
            if sub.regex.is_match(topic) {
                (sub.func)(self.state.as_ref(), topic, &json_value);
            }
        }

    }
}


fn spawn_mqtt_thread<T: MQTTState + 'static + Sync + Send + Clone>(mut eventloop: EventLoop, mqtt_server: Arc<MQTTServer<T>>) {
    task::spawn(async move {
        while let Ok(event) = eventloop.poll().await {
            match event {
                Incoming(packet) => {
                    match packet {
                        Publish(publish) => {
                            let topic = publish.topic.as_str();
                            let body = std::str::from_utf8(&publish.payload).unwrap();
                            mqtt_server.receive(topic, body).await;
                        },
                        Connect(_connect) => {
                            println!("MQTT Connected");
                            mqtt_server.resubscribe().await;
                        },
                        Disconnect => {
                            println!("MQTT Disconnected");
                        },
                        _ => {}
                    }
                },
                Outgoing(_outgoing) => { },
            }
        } 
    });
}