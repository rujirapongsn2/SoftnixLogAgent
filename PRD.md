# **Product Requirements Document (PRD)**  
## **Softnix Log Collector Agent (Rust-based)**  
**Version:** 1.0  
**Author:** Softnix Technology  
**Purpose:** ระบบ Log Collector Agent สำหรับเก็บ log จาก OS / Application และส่งต่อไปยัง Centralized Log Server พร้อมด้วยความสามารถด้าน Threat Intelligence Enrichment แบบ Offline + Optional Online

---

# **1. Overview**

Softnix Log Collector Agent เป็น Agent ที่พัฒนาด้วยภาษา **Rust** เพื่อให้ได้ความเร็วสูง, ใช้ทรัพยากรต่ำ และทำงานได้บนหลายระบบปฏิบัติการ โดยมีความสามารถหลักดังนี้:

- เก็บ log จากแหล่งต่าง ๆ  
- แปลงและ enrich log ก่อนส่งออก  
- ส่ง log ไปยัง Centralized Log Server ผ่าน Syslog  
- รองรับ Threat Intelligence Enrichment แบบออฟไลน์ (Offline TI DB)  
- รองรับ Threat Intelligence Online แบบ optional  
- ทำงานแบบ command หรือแบบ service/daemon  

---

# **2. Goals**

- รองรับการใช้งานบนหลายระบบปฏิบัติการ (Linux, Windows, macOS)  
- ใช้ resource ต่ำ (lightweight)  
- มีความยืดหยุ่นในการปรับแต่ง (modular config)  
- ปลอดภัยและใช้งานง่าย  
- เสริมประสิทธิภาพการตรวจจับภัยคุกคามด้วยการ Enrich Log ผ่าน Threat Intelligence  

---

# **3. Architecture Overview**

### **3.1 ส่วนประกอบหลัก**

1. **Input Modules**  
   - File Tail (`/var/log/*.log`)  
   - journald (Linux)  
   - TCP/UDP Listener  
   - Windows Event Log (เลือก channel ได้ เช่น Application, System, Security หรือ custom)  
   - Stdout/Stderr  

2. **Parser / Normalizer**  
   - แปลง log → โครงสร้างกลาง  
   - Extract IoC เช่น IP, Domain, URL, File Hash  

3. **Threat Intelligence Enrichment**
   - **Offline TI Database (SQLite)**  
   - Optional: Online TI via HTTP API  
   - ตรวจสอบ IoC เพื่อเพิ่ม metadata เช่น  
     - `ti_malicious`  
     - `ti_level`  
     - `ti_source`  
     - `ti_category`  

4. **Processing Layer**
   - Filtering  
   - Tagging  
   - Enrichment  

5. **Output Modules**
   - Syslog (UDP / TCP / TLS)  
   - RFC 3164 / RFC 5424  
   - JSON output (optional)  

6. **Configuration System**  
   - TOML หรือ YAML  

7. **Runtime Mode**
   - Run แบบ command  
   - Run แบบ background service/daemon  
   - รองรับ systemd, Windows Service, macOS launchd  

---

# **4. Threat Intelligence (TI) System**

## **4.1 Offline TI Database (Primary Mode)**

Agent จะใช้ SQLite เป็นฐานข้อมูลสำหรับ Threat Intelligence แบบออฟไลน์ โดยเหตุผล:

- จัดเก็บง่าย  
- เสถียร, เชื่อถือได้  
- ข้ามแพลตฟอร์ม  
- อัปเดตง่าย  
- ไม่มี dependency ภายนอก  
- latency ต่ำ  

### **โครงสร้างฐานข้อมูล (ตัวอย่าง)**

```sql
CREATE TABLE threat_intel (
    id INTEGER PRIMARY KEY,
    ioc_type TEXT,             -- เช่น IP, DOMAIN, URL, HASH
    ioc_value TEXT UNIQUE,     
    threat_level TEXT,         -- high, medium, low
    category TEXT,             -- เช่น botnet, phishing
    source TEXT,               
    last_updated TIMESTAMP
);

4.2 Memory Pre-Loading (สำหรับเครื่องประสิทธิภาพสูง)

เพื่อเพิ่มประสิทธิภาพ:
   •  เมื่อ Agent เริ่มทำงาน ให้ โหลดข้อมูลจาก SQLite ขึ้นมาเก็บใน Memory
   •  ใช้โครงสร้างข้อมูล เช่น HashSet หรือ HashMap
   •  ทำให้ lookup IoC เป็น O(1)

ประโยชน์
   •  เร็วขึ้นมาก
   •  ลด I/O จาก disk
   •  เหมาะสำหรับเครื่องที่มี RAM มาก

Config Example
[threat_intel]
enable_offline = true
enable_memory_preload = true
sqlite_path = "/etc/softnix/ti.db"

4.3 Optional Online Threat Intelligence (Secondary Mode)
   •  เปิดเป็น optional เพื่อป้องกันปัญหา latency / rate limit
   •  ทำ caching (LRU)
   •  retry, timeout
   •  ใช้เมื่อไม่เจอใน offline DB

Config Example

[threat_intel.online]
enabled = false
api_url = ""
api_key = ""
timeout_ms = 200

5. Installation / Deployment Modes

Agent รองรับ 2 รูปแบบ:

5.1 Command-line Mode

เหมาะสำหรับทดสอบและ manual run:

softnix_agent --config /etc/softnix/agent.toml

5.2 Service / Daemon Mode

Linux (systemd)

[Unit]
Description=Softnix Log Collector Agent

[Service]
ExecStart=/usr/local/bin/softnix_agent --config /etc/softnix/agent.toml
Restart=always

[Install]
WantedBy=multi-user.target

Windows
   •  รันเป็น Windows Service ผ่าน winsvc crate

macOS
   •  รองรับ launchd daemon

   6. Performance Requirements
   •  CPU usage ต่ำ
   •  Memory ต่ำในโหมดปกติ
   •  โหมด memory preloaded ใช้ RAM เพิ่มขึ้นแต่ได้ความเร็วสูง
   •  รองรับ throughput สูง
   •  Non-blocking I/O ผ่าน tokio

⸻

7. Security Requirements
   •  ไม่มีการเปิด port ที่ไม่จำเป็น
   •  Syslog TLS (optional)
   •  Offline TI DB ไม่มีการส่งข้อมูลออกภายนอก
   •  Online TI เป็น optional
   •  Config file permission: 600

8. Example Configuration (TOML)

[agent]
mode = "service"
log_level = "info"

[inputs.files]
type = "file_tail"
paths = ["/var/log/nginx/access.log"]

[inputs.windows_event_log]
type = "windows_event_log"
channels = ["Application", "Security"]
bookmark_persist_path = "C:\\ProgramData\\Softnix\\agent.wevtbookmark"

[outputs.syslog]
type = "syslog_udp"
server = "10.10.1.20:514"
format = "rfc3164"

[threat_intel]
enable_offline = true
enable_memory_preload = true
sqlite_path = "/etc/softnix/ti.db"

[threat_intel.online]
enabled = false
api_url = ""
api_key = ""


9. Future Enhancements
   •  Local TI auto-update
   •  Hash-based malware intelligence
   •  MITRE ATT&CK mapping
   •  ML anomaly detection

⸻

10. Summary

Softnix Log Collector Agent:
   •  Rust-based → เร็ว, เบา, ปลอดภัย
   •  Multi-platform
   •  Offline TI DB (SQLite) + Optional Online TI
   •  Memory Preload สำหรับเร่งความเร็ว
   •  เหมาะกับเครื่องหลายสเปก
   •  ส่งออก Syslog มาตรฐาน

11. Web-based GUI Configuration Service

Softnix Log Collector Agent จะมีโมดูลเสริมสำหรับ GUI แบบ Web Interface เพื่อช่วยอำนวยความสะดวกในการจัดการ agent โดยเฉพาะสำหรับผู้ดูแลระบบที่ต้องการตั้งค่าอย่างง่ายและควบคุมตัว agent ได้โดยไม่ต้องเปิดไฟล์ config ด้วยตนเอง

⸻

11.1 Objectives
	•	ให้ผู้ใช้ปรับแต่ง configuration ผ่านกราฟิกหน้าเว็บได้ง่าย
	•	ลดความผิดพลาดจากการแก้ไขไฟล์ config ด้วยมือ
	•	ควบคุม service ของ Agent (start/stop/reload) ได้จาก UI
	•	ควบคุมสิทธิ์การเข้าถึง และจำกัดให้เข้าผ่าน localhost เป็นค่าเริ่มต้น
	•	รองรับการใช้งานได้ทั้ง Linux และ Windows

⸻

11.2 Architecture

GUI ประกอบด้วย 2 ส่วนหลัก:

1. Embedded HTTP Admin Server (Rust)
	•	เป็น Web server ขนาดเล็กที่ฝังใน agent หรือรันคู่กับ agent
	•	รองรับ REST API สำหรับจัดการ config และ service control
	•	รันบน 127.0.0.1 (localhost) เป็นค่า default
	•	ให้เปลี่ยน bind address ได้ (เช่น 0.0.0.0) ผ่าน config โดยผู้ดูแลระบบ

2. Web UI (HTML/JS)
	•	โหลดจากภายใน agent (embedded) หรือจาก static directory
	•	แสดงผลเป็นฟอร์มเพื่อแก้ไข config และปุ่มควบคุม agent

⸻

11.3 Default Security Model

เป็นแนวทางความปลอดภัยเบื้องต้น:

ค่า Default
	•	เปิด GUI ผ่าน:
http://127.0.0.1:8080
	•	ไม่อนุญาต remote access โดยค่า default
	•	แนะนำให้ตั้ง authentication: token หรือ username/password

Config Example:
[web_admin]
enabled = true
bind_address = "127.0.0.1:8080"
require_auth = true
auth_token = "change_me"

ผู้ดูแลระบบสามารถเปลี่ยน bind_address ได้ หากต้องการให้เข้าระยะไกล เช่น:
bind_address = "0.0.0.0:8080"

11.4 GUI Features

11.4.1 Configuration Editor
	•	อ่านค่า config ปัจจุบันจากไฟล์ เช่น /etc/softnix/agent.toml
	•	แสดงผลเป็นฟอร์ม (Syslog settings, TI settings, input modules)
	•	ผู้ใช้แก้ไขได้ และกด “Save”
	•	เมื่อบันทึกแล้ว GUI จะ:
	•	เขียนไฟล์ config ใหม่
	•	เรียก service reload (ผ่าน API หรือ systemctl)

⸻

11.4.2 Service Control Panel

GUI จะมีปุ่มพื้นฐาน:
	•	Start Agent
	•	Stop Agent
	•	Restart Agent
	•	Reload Configuration

Linux (systemd)

GUI จะเรียกผ่าน backend เช่น:
systemctl start softnix_agent
systemctl stop softnix_agent
systemctl restart softnix_agent
systemctl reload softnix_agent

โดยการตั้งค่า sudoers เฉพาะเจาะจง เช่น:
softnixgui ALL=NOPASSWD: /bin/systemctl start softnix_agent

Windows (Service Manager)

GUI จะใช้ WinAPI หรือสั่งผ่าน sc.exe เช่น:
sc start SoftnixAgent
sc stop SoftnixAgent

sc start SoftnixAgent
sc stop SoftnixAgent

11.5 Admin REST API

ตัวอย่าง API endpoints:
GET    /config/get
POST   /config/save

POST   /service/start
POST   /service/stop
POST   /service/restart
POST   /service/reload

GET    /health
GET    /version
รูปแบบ Payload:

{
  "status": "ok",
  "message": "Agent restarted"
}

11.6 Deployment Options

รัน GUI เป็นส่วนหนึ่งของ Agent
	•	Agent process มี embedded web admin server
	•	เรียก /admin/... ผ่าน localhost
	•	เหมาะกับเครื่องที่มี agent ตัวเดียว

11.7 OS Support

Linux
	•	ใช้ systemd สำหรับ lifecycle control
	•	Commands: systemctl start/stop/reload
	•	GUI สามารถทำงานเป็น root หรือมี sudoers limit

Windows
	•	ใช้ Windows Service Control Manager
	•	GUI รันด้วยสิทธิ์ Administrator

11.8 UX Guidelines สำหรับ GUI
	•	ใช้ layout แบบ Clean (sidebar + main panel)
	•	แสดงสถานะ agent แบบ real-time เช่น
	•	running / stopped
	•	config loaded
	•	last reload
	•	มีให้ test connection เช่น “Test Syslog Server”
	•	มี log panel แสดง error ของ agent แบบ live tail
