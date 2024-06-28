use dotenv::dotenv;
use libsql::{Builder, Connection};
use std::env;
use sysinfo::{Disks, System};

struct SystemInfo<'a> {
    system_name: &'a str,
    system_host_name: &'a str,
}

struct DiskInfo {
    system_total_space: f64,
    system_available_space: f64,
    system_used_space: f64,
}

impl DiskInfo {
    fn bytes_to_gb(bytes: u64) -> f64 {
        bytes as f64 / 1024f64 / 1024f64 / 1024f64
    }
}

#[tokio::main]
async fn main() -> Result<(), libsql::Error> {
    let conn = init_db().await.unwrap();

    let mut sys = System::new_all();
    sys.refresh_all();

    let system_name = System::name().unwrap_or_default().to_string();
    let system_host_name = System::host_name().unwrap_or_default().to_string();

    let system_info = SystemInfo {
        system_name: system_name.as_str(),
        system_host_name: system_host_name.as_str(),
    };

    let disks = Disks::new_with_refreshed_list();
    let get_disk_info = get_disk_info(disks);

    let mut disk_info = DiskInfo {
        system_total_space: 0.0,
        system_available_space: 0.0,
        system_used_space: 0.0,
    };

    if let Some(d) = get_disk_info {
        let system_used_space = d.system_used_space;
        let system_total_space = d.system_total_space;
        let system_available_space = d.system_available_space;

        disk_info = DiskInfo {
            system_total_space,
            system_available_space,
            system_used_space,
        };
    }

    insert_into_db(&conn, system_info, disk_info).await?;
    Ok(())
}

async fn insert_into_db(
    conn: &Connection,
    system: SystemInfo<'_>,
    disk: DiskInfo,
) -> Result<(), libsql::Error> {
    conn.execute(
        "INSERT INTO info (system_name, system_host_name, system_total_space, system_available_space, system_used_space) VALUES (?1, ?2, ?3, ?4, ?5)",
        [
            system.system_name,
            system.system_host_name,
            disk.system_total_space.to_string().as_str(),
            disk.system_available_space.to_string().as_str(),
            disk.system_used_space.to_string().as_str(),
        ],
    )
    .await
    .unwrap_or_else(|e| {
        panic!("Error: {:?}", e);
    });
    Ok(())
}

async fn init_db() -> Result<Connection, libsql::Error> {
    dotenv().ok();

    let url = env::var("LIBSQL_URL").expect("LIBSQL_URL must be set");
    let token = env::var("LIBSQL_AUTH_TOKEN").unwrap_or_default();

    let db = Builder::new_remote(url, token).build().await?;
    let conn = db.connect().unwrap();
    Ok(conn)
}

fn get_disk_info(disks: Disks) -> Option<DiskInfo> {
    for disk in &disks {
        if let Some(name) = disk.name().to_str() {
            if name.contains("1p6")
                && disk
                    .mount_point()
                    .to_str()
                    .unwrap_or_else(|| panic!("Error getting mount point"))
                    .contains("/home")
            {
                let total_usage = disk.total_space() - disk.available_space();

                return Some(DiskInfo {
                    system_total_space: DiskInfo::bytes_to_gb(disk.total_space()),
                    system_available_space: DiskInfo::bytes_to_gb(disk.available_space()),
                    system_used_space: DiskInfo::bytes_to_gb(total_usage),
                });
            }
        }
    }
    None
}
