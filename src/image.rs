use std::{env, fs::File, io::Write, time::SystemTime};

use chrono::{DateTime, Utc};
use tokio::task;



pub fn spawn_imager(cam: String, ip: String) {
    task::spawn(async move {
        let now: DateTime<Utc> = SystemTime::now().into();
        let image_folder = env::var("IMAGE_FOLDER").unwrap();
        let url = format!("http://{}/picture", ip);
        let filename = format!("{}-{}.jpg", cam, now.format("%Y-%m-%d-%H-%M-%S"));
        let filepath = format!("{}/{}", image_folder, filename);
        println!("Getting images from {} at {} and writting it to {}", cam, ip, filepath);
        let resp = reqwest::get(url).await.unwrap();
        let mut file = File::create(filepath).unwrap();
        let bytes = resp.bytes().await.unwrap();
        file.write_all(&bytes).unwrap();
    });
}