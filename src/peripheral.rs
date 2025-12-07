use std::time::Duration;

use anyhow::Result;
use ble_peripheral_rust::{
    Peripheral, PeripheralImpl,
    gatt::{
        characteristic::Characteristic,
        peripheral_event::{PeripheralEvent, RequestResponse, WriteRequestResponse},
        properties::{AttributePermission, CharacteristicProperty},
        service::Service,
    },
};
use log::{info, warn};
use tokio::sync::mpsc;

use crate::constants::{CHARACTERISTIC_UUID, SERVICE_UUID};

fn create_service() -> Service {
    Service {
        uuid: SERVICE_UUID,
        primary: true,
        characteristics: [Characteristic {
            uuid: CHARACTERISTIC_UUID,
            properties: [CharacteristicProperty::Write].to_vec(),
            permissions: [AttributePermission::Writeable].to_vec(),
            descriptors: [].to_vec(),
            value: None,
        }]
        .to_vec(),
    }
}

pub async fn handle_peripheral() -> Result<()> {
    let (sender_tx, mut receiver_rx) = mpsc::channel::<PeripheralEvent>(256);

    let mut peripheral = Peripheral::new(sender_tx).await?;

    while !peripheral.is_powered().await? {
        info!("Bluetooth アダプタの電源をオンにしてください");
        tokio::time::sleep(Duration::from_secs(5)).await;
    }

    peripheral.add_service(&create_service()).await?;

    info!("アドバタイズを開始します");
    peripheral
        .start_advertising("BLE Dual Role", [SERVICE_UUID].as_ref())
        .await?;

    while let Some(event) = receiver_rx.recv().await {
        match event {
            PeripheralEvent::WriteRequest { value, responder, .. } => {
                match value.get(0..4).and_then(|b| b.try_into().ok()) {
                    Some(bytes) => info!("受信: {}", i32::from_le_bytes(bytes)),
                    None => warn!("無効なデータ長: {} バイト", value.len()),
                }

                responder
                    .send(WriteRequestResponse {
                        response: RequestResponse::Success,
                    })
                    .unwrap();
            }
            _ => {
                warn!("未処理のイベントが発生しました: {:?}", event);
            }
        }
    }

    Ok(())
}
