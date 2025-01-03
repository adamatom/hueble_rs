use bluer::{gatt::remote::Characteristic, Address as BluerAddress, Device, Session};
use std::{collections::HashMap, error::Error, str::FromStr, sync::Arc};
use tokio::sync::RwLock;
use uuid::Uuid;

pub struct GattManipulator {
    address: String,
    device: Device,
    /// Cache for the characteristics objects, not their values
    ch_cache: RwLock<HashMap<Uuid, Arc<Characteristic>>>,
}

impl GattManipulator {
    pub async fn new(address: &str) -> Result<Self, Box<dyn Error>> {
        // Create a new BlueR session
        let session = Session::new().await?;

        // Get default adapter (like "hci0"), ensure it's powered
        let adapter = session.default_adapter().await?;
        adapter.set_powered(true).await?;

        // Get the device from mac
        let target_addr = BluerAddress::from_str(address)?;
        let device = adapter.device(target_addr)?;

        // Connect if not already
        device.ensure_connected().await;
        println!("Connected to device at {}", &address);

        Ok(Self {
            address: address.to_string(),
            device,
            ch_cache: RwLock::new(HashMap::new()),
        })
    }

    pub async fn write_characteristic(&self, ch_uuid: &Uuid, data: &[u8]) {
        loop {
            self.device.ensure_connected().await;

            match self.resolve_characteristic(ch_uuid).await {
                None => return, // not found in device
                Some(ch) => match ch.write(data).await {
                    Ok(_) => return,    // successfully written
                    Err(_) => continue, // error while writing data, retry from the beginning
                },
            }
        }
    }

    pub async fn read_characteristic(&self, ch_uuid: &Uuid) -> Option<Vec<u8>> {
        loop {
            self.device.ensure_connected().await;

            match self.resolve_characteristic(ch_uuid).await {
                None => return None, // not found in device
                Some(ch) => match ch.read().await {
                    Ok(data) => return Some(data),
                    Err(_) => continue, // Error while reading data, retry from the beginning
                },
            }
        }
    }

    pub async fn disconnect(&self) {
        println!("Disconnected from {}", &self.address);
        let _ = self.device.disconnect().await;
    }

    async fn resolve_characteristic(&self, uuid: &Uuid) -> Option<Arc<Characteristic>> {
        loop {
            self.device.ensure_connected().await;

            if let Some(ch) = self.read_cache(uuid).await {
                return Some(ch);
            }

            // Enumerate GATT services & characteristics, on error loop again to try again
            match self.scan_services_for_characteristic(uuid).await {
                Ok(Some(ch)) => return Some(ch),
                Ok(None) => return None,
                Err(_) => {}
            }
        }
    }

    async fn read_cache(&self, uuid: &Uuid) -> Option<Arc<Characteristic>> {
        let cache = self.ch_cache.read().await;
        cache.get(uuid).cloned()
    }

    async fn write_cache(&self, uuid: &Uuid, arc_ch: Arc<Characteristic>) {
        let mut cache = self.ch_cache.write().await;
        cache.insert(*uuid, arc_ch);
    }

    async fn scan_services_for_characteristic(
        &self,
        ch_uuid: &Uuid,
    ) -> Result<Option<Arc<Characteristic>>, Box<dyn Error>> {
        for svc in self.device.services().await? {
            for ch in svc.characteristics().await? {
                if ch.uuid().await? != *ch_uuid {
                    continue;
                }

                self.write_cache(ch_uuid, Arc::new(ch)).await;
                return Ok(self.read_cache(ch_uuid).await);
            }
        }
        Ok(None)
    }
}

trait EnsureConnected {
    async fn ensure_connected(&self);
}

impl EnsureConnected for bluer::Device {
    async fn ensure_connected(&self) {
        loop {
            if let Ok(is_connected) = self.is_connected().await {
                if is_connected || self.connect().await.is_ok() {
                    break;
                }
            }
        }
    }
}
