use actix_web::{get, web, Error, HttpResponse};
use maud::{html, DOCTYPE};

use crate::state::AppState;

struct CameraUIData {
    name: String,
    ip: String,
    stream_id: u8
}

#[get("/")]
async fn index(state: web::Data<AppState>,) -> Result<HttpResponse, Error> {
    let cams = state.for_all_cameras(|cam| {
        CameraUIData { name: cam.name.to_owned(), ip: cam.ip.to_owned(), stream_id: cam.stream_id }
    }).await;

    let html = html! {
        (DOCTYPE)
        html {
            head {
                script src="js/main.js" {}
                link rel="stylesheet" href="css/main.css" {}
            }
            body {
                @for cam_info in &cams {
                    div class="camcontainer" ip=(cam_info.ip) {
                        div class="caminfo" {
                            div class="camname" {(cam_info.name)}
                            div class="camip" {(cam_info.ip)}
                        }
                        div class="camimg" {
                            img id=(format!("{}img", cam_info.name)) {}
                        }
                        div class="camctl" {
                            div class="camctlcol" {
                                div class="camctlitem" {
                                    div class="camctltitle" {"Filter"}
                                    div class="camctlinput" {
                                        input type="checkbox" onclick=(format!("filter('{}', this.checked)", cam_info.name)) {}
                                    }
                                }  
                                div class="camctlitem" {
                                    div class="camctltitle" {"IR"}
                                    div class="camctlinput" {
                                        input type="checkbox" onclick=(format!("ir('{}', this.checked)", cam_info.name)) {}
                                    }
                                }  
                                div class="camctlitem" {
                                    div class="camctltitle" {"Flip"}
                                    div class="camctlinput" {
                                        input type="checkbox" onclick=(format!("flip('{}', this.checked)", cam_info.name)) {}
                                    }
                                }                                                                                          
                            }
                            div class="camctlcolsep" { }
                            div class="camctlcol" {
                                button id=(format!("{}httpstreambut", cam_info.name)) class="camctlbutton" onclick=(format!("streamToggle('{}', 'http', 0)", cam_info.name)) { "HTTP Stream" }
                                button id=(format!("{}udpstreambut", cam_info.name)) class="camctlbutton" onclick=(format!("streamToggle('{}', 'udp', {})", cam_info.name, cam_info.stream_id)) { "UDP Stream" }
                                button id=(format!("{}movementbut", cam_info.name)) class="camctlbutton" onclick=(format!("showMovement('{}')", cam_info.name)) { "Movement" }
                            }
                        }
                    }    
                    script { 
                        (format!("addCam('{}', '{}');", cam_info.name, cam_info.ip)) 
                        (format!("setUdpIP('{}');", "192.168.1.130")) 
                    }
                }
                @if cams.len() == 0 {
                    div class="message" {
                        p {("No cams available")}
                    }
                }
            }
        }
    };
    Ok(HttpResponse::Ok().body(html.0))
}
