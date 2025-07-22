use std::{env, future::Future, pin::Pin, sync::Arc, time::Duration};

use regex::Regex;
use rumqttc::{AsyncClient, Event::{Incoming, Outgoing}, EventLoop, MqttOptions, Packet::{Publish, Connect, Disconnect}};
use serde_json::Value;
use tokio::{sync::Mutex, task}; 
use log::info;
type AsyncFuncType<T> = Box<dyn Fn(T, String, Value) -> Pin<Box<dyn Future<Output=()> + Send>> + Send + Sync>;

pub trait MQTTState {
    fn set_mqtt_client(&self, client: AsyncClient) -> impl Future<Output=()> + Send;
    fn mqtt_client_subscribe(&self, topic: &str) -> impl Future<Output=()> + Send;
    fn mqtt_publish(&self, topic: &str, body: &str) -> impl Future<Output=()> + Send;
}

pub struct Subscription<ST> 
where 
    ST: MQTTState + Sync + Send + Clone,
{
    topic: String,
    regex: Regex,
    func: AsyncFuncType<ST>
}

impl<ST> Subscription<ST>  
where 
    ST: MQTTState + Sync + Send + Clone,
{
    fn run<'a>(&'a self, state: ST, topic: String, body: Value) -> impl Future<Output=()> + 'a + Send {
        async {
            (self.func)(state, topic, body).await;
        }
    }
}

#[derive(Clone)]
pub struct MQTTServer<ST> 
where
    ST: MQTTState + Sync + Send + Clone,
{
    state: Arc<ST>,
    subs: Arc<Mutex<Vec<Subscription<ST>>>>
}

impl<ST> MQTTServer<ST> 
where
    ST: MQTTState + Sync + Send + Clone + 'static,
{
    pub async fn new(state: ST) -> Self {
        let mqtt_ip = env::var("MQTT_BROKER").unwrap();
        let mut mqttoptions = MqttOptions::new("camserver", &mqtt_ip, 1883);
        mqttoptions.set_keep_alive(Duration::from_secs(5));
        let (client, eventloop) = AsyncClient::new(mqttoptions, 10);
        state.set_mqtt_client(client).await;
        let arcstate = Arc::new(state);
        let arcsubs = Arc::new(Mutex::new(Vec::new()));
        let ret: MQTTServer<ST> = MQTTServer {
            state: arcstate.clone(),
            subs: arcsubs.clone()
        };
        spawn_mqtt_thread(eventloop, ret.clone());
        ret
    }

    pub async fn subscribe<F, Fut>(&self, topic: &str, f: F)
    where 
        F: Fn(ST, String, Value) -> Fut + Send + Sync + 'static,
        Fut: Future<Output=()> + Send + 'static
    {
        info!("MQTT Subscribing to {}", &topic);
        let mut subs_locked = self.subs.lock().await;
        let retext = topic
            .replace("/", "\\/")
            .replace("+", "[a-zA-Z0-9_]*")
            .replace("#", "[a-zA-Z0-9_\\/]*");
        let regex = Regex::new(retext.as_str()).unwrap();
        let subs = Subscription {
            topic: topic.to_string(), 
            regex, 
            func: Box::new(move |s, t, b| Box::pin(f(s, t, b)))
        };
        subs_locked.push(subs);
        self.state.mqtt_client_subscribe(topic).await;
    }

    pub async fn resubscribe(&self) {
        let topics: Vec<String> = self.subs.lock().await.iter().map(|s| {s.topic.to_string()}).collect();
        for topic in topics {
            self.state.mqtt_client_subscribe(&topic).await;
        }
    }

    fn receive<'a>(&'a self, topic: String, body: Value) -> impl Future<Output = ()> + Send + 'a {
        async move {
            let subs = self.subs.lock().await;
            for sub in subs.iter() {
                if sub.regex.is_match(&topic) {
                    let st = (*self.state).clone();
                    sub.run(st, topic.clone(), body.clone()).await;
                }
            }
        }
    } 

}



fn spawn_mqtt_thread<ST>(mut eventloop: EventLoop, mqtt_server: MQTTServer<ST>) 
where
    ST: MQTTState + Sync + Send + Clone + 'static,
{
    task::spawn(async move {
        info!("MQTT Server Started");
        while let Ok(event) = eventloop.poll().await {
            match event {
                Incoming(packet) => {
                    match packet {
                        Publish(publish) => {
                            let topic = publish.topic.clone();
                            let body = std::str::from_utf8(&publish.payload).unwrap();
                            let json_value: Value = serde_json::from_str(body).unwrap();
                            mqtt_server.receive(topic, json_value).await;
                        },
                        Connect(_connect) => {
                            info!("MQTT Connected");
                            mqtt_server.resubscribe().await;
                        },
                        Disconnect => {
                            info!("MQTT Disconnected");
                        },
                        _ => {}
                    }
                },
                Outgoing(_outgoing) => { },
            }
        } 
    });
}