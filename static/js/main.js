var cams = {};
var udpip = "";

function addCam(cam, ip) {
    cams[cam] = {
        ip: ip,
        streaming: false
    };
}

function setUdpIP(ip) {
    udpip = ip;
}

function streamToggle(cam, type, stream_id) {
    var butt = document.getElementById(cam + type + "streambut");
    var img = document.getElementById(cam + "img");
    if(cams[cam].streaming) {
        cams[cam].streaming = false;
        if(type == 'http') {
            butt.innerHTML = "HTTP Stream";
            img.src = "";
        } else if(type == 'udp') {
            butt.innerHTML = "UDP Stream";
            img.src = "";
            postState(cam, {streamto:"", streamid:0});
        }
    } else {
        butt.innerHTML = "Stop";
        cams[cam].streaming = true;
        if(type == 'http') {
            img.src = "http://" + cams[cam].ip + "/stream";
        } else if(type == 'udp') {
            img.src = "/" + cam + "/stream";
            postState(cam, {streamto: udpip, streamid: stream_id});
        }
    }
}

function filter(cam, val) {
    postState(cam, {filter: val ? 1 : 0});
}

function ir(cam, val) {
    postState(cam, {ir: val ? 1 : 0});
}

function flip(cam, val) {
    postState(cam, {flip: val ? 1 : 0});
}

function imgsize(cam, val) {
    postState(cam, {size: parseInt(val)});
}

async function postState(cam, body) {
    post("/api/" + cam + "/state", body);
}

async function post(url, body) {
    var resp = await window.fetch(url, {method:"POST", headers:{"Content-Type":"application/json"}, body:JSON.stringify(body)}); 
}

async function showMovement(cam) {
    var resp = await window.fetch("/api/" + cam + "/movements", {method:"GET"});
    var json = await resp.json();
    var list = json.movements;
    list.sort((a, b) => b.timestamp.localeCompare(a.timestamp));
    var content = "<div class='moveframe'>" + list.map(m => "- " + new Date(m.timestamp)).join('<br>') + "</div>";
    var div = document.createElement("div");
    div.className = "moveoverlay";
    div.id = "moveoverlay";
    div.innerHTML = content;
    div.onclick = (event) => {document.body.removeChild(document.getElementById("moveoverlay"));};
    document.body.appendChild(div);
}