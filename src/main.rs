mod state;
mod mqtt;
mod http;
mod image;
mod stream;
mod utils;

use std::{io, sync::{mpsc::{self}, Arc}};
use actix_files as af;
use actix_web::{get, http::Error, post, web::{self, Bytes, Data}, App, HttpResponse, HttpServer};
use chrono::{DateTime, Utc};
use http::index;
use image::spawn_imager;
use log::info;
use mqtt::{MQTTServer, MQTTState};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use state::AppState;
use stream::{StreamReceiver};
use async_stream::stream;


#[derive(Serialize, Deserialize, Debug)]
struct StateRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ir: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flip: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub streamto: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub streamid: Option<u8>
}

#[post("/api/{cam}/state")]
async fn post_state(state: web::Data<AppState>, cam: web::Path<String>, body: web::Json<StateRequest>) -> Result<HttpResponse, Error> {
    let topic = format!("home/cams/{}/cmd", &cam);
    let mqtt_body = serde_json::to_string(&body).unwrap();
    state.mqtt_publish(&topic, &mqtt_body).await;
    Ok(HttpResponse::Ok().finish())
}


#[derive(Serialize, Deserialize, Debug)]
struct MovementItemResponse {
    timestamp: DateTime<Utc>
}

#[derive(Serialize, Deserialize, Debug)]
struct MovementResponse {
    movements: Vec<MovementItemResponse>
}

#[get("/api/{cam}/movements")]
async fn get_movements(state: web::Data<AppState>, cam_name: web::Path<String>) -> Result<HttpResponse, Error> {
    let mvts_opt: Option<Vec<MovementItemResponse>> = state.for_camera(cam_name.as_str(), |cam| {
        cam.moves.iter().map(|m| MovementItemResponse{ timestamp: *m}).collect()
    }).await;
    let resp = MovementResponse { movements: mvts_opt.unwrap_or(vec![]) };
    Ok(HttpResponse::Ok().json(resp))
}

fn prepare_http_bytes(data: Arc<Vec<u8>>) -> Bytes {
    let http_len = &format!("Content-Length: {}\r\n\r\n", data.as_ref().len())[..];
    let data_slice = &data.as_ref()[..];
    let d = ["\r\n--123456789000000000000987654321\r\n".as_bytes(), "Content-Type: image/jpeg\r\n".as_bytes(), http_len.as_bytes(), data_slice].concat();
    Bytes::from(d)
}
 
#[get("/{cam}/stream")]
async fn get_stream(state: web::Data<AppState>, cam_name: web::Path<String>) -> Result<HttpResponse, Error> {
    let (tx, rx) = mpsc::channel::<Arc<Vec<u8>>>();
    state.for_mut_camera(cam_name.as_str(), |cam| {
        cam.add_sender(tx.clone());
    }).await;
    Ok(HttpResponse::Ok()
        .content_type("multipart/x-mixed-replace;boundary=123456789000000000000987654321")
        .streaming(stream! {
            loop {
                match rx.recv() {
                    Ok(data) => yield Ok(prepare_http_bytes(data)),
                    Err(err) => yield Err(err),
                }
            }
        })
    )
}

async fn mqtt_cam_stat(state: AppState, topic: String, body: Value) {
    let name = &topic[10..topic.len() - 5];
    let ip = body["ip"].as_str().unwrap();
    let lum = if let Some(r) = body["lum"].as_u64() {r as u8} else {0};
    info!("Cam Stat {}: ip {}, lum {}", name, ip, lum);
    state.set_camera_stat(name, ip, lum).await;
}

async fn mqtt_cam_move(state: AppState, topic: String, body: Value) {
    let name = &topic[10..topic.len() - 5];
    info!("Cam move {}: {}", name, body);
    state.for_mut_camera(name, |cam| {
        cam.record_movement();
        spawn_imager(cam.name.to_owned(), cam.ip.to_owned());
    }).await;
}

#[actix_web::main]
async fn main() -> io::Result<()> {
    env_logger::init();
    let state = AppState::new();

    let mqtt_server = MQTTServer::new(state.clone()).await;
    mqtt_server.subscribe("home/cams/+/stat", mqtt_cam_stat).await;
    mqtt_server.subscribe("home/cams/+/move", mqtt_cam_move).await;

    StreamReceiver::init(state.clone());
    
    HttpServer::new(move || {
        App::new()
        .app_data(Data::new(state.clone()))
        .service(index)
        .service(post_state)
        .service(get_movements)
        .service(get_stream)
        .service(af::Files::new("/css", "./static/css").show_files_listing())
        .service(af::Files::new("/img", "./static/img").show_files_listing())
        .service(af::Files::new("/js", "./static/js").show_files_listing())
    })
    .max_buffer_size(10_000)
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}

