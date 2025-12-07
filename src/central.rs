use std::time::Duration;

use anyhow::{Result, anyhow};
use btleplug::{
    api::{BDAddr, Central, CentralState, Manager as _, Peripheral as _, ScanFilter, WriteType},
    platform::{Adapter, Manager, Peripheral},
};
use log::{error, info, warn};
use rand::{Rng, SeedableRng, rngs::StdRng};
use tokio::time;

use crate::constants::{CHARACTERISTIC_UUID, SERVICE_UUID};

async fn get_central() -> Result<Adapter> {
    let manager = Manager::new().await?;

    manager
        .adapters()
        .await
        .map_err(|_| anyhow!("アダプターの取得に失敗しました。"))?
        .into_iter()
        .next()
        .ok_or_else(|| anyhow!("アダプターが見つかりませんでした。"))
}

async fn find_peripheral(central: &Adapter, target_address: &BDAddr) -> Result<Peripheral> {
    loop {
        if let Some(p) = central
            .peripherals()
            .await?
            .into_iter()
            .find(|p| p.address() == *target_address)
        {
            return Ok(p);
        }
        time::sleep(Duration::from_secs(2)).await;
    }
}

pub async fn handle_central(target_address: &BDAddr) -> Result<()> {
    let central = get_central().await?;

    while central.adapter_state().await? != CentralState::PoweredOn {
        // peripheral.rs でも同様の処理を行っているため、ログ出力は省略します。
        tokio::time::sleep(Duration::from_secs(5)).await;
    }

    central
        .start_scan(ScanFilter {
            services: [SERVICE_UUID].to_vec(),
        })
        .await?;

    let mut rng = StdRng::from_os_rng();
    // 一度ペリフェラルに接続するかすると相手が起動していなくともキャラクタリスティックの取得まで進むことがあり、キャラクタリスティックの取得に失敗してしまうため、無限ループで再試行します。
    loop {
        let peripheral = find_peripheral(&central, target_address).await?;

        if (peripheral.connect().await).is_err() {
            error!("ペリフェラルへの接続に失敗しました。再試行します...");
            time::sleep(Duration::from_secs(5)).await;
            continue;
        }

        peripheral
            .discover_services()
            .await
            .map_err(|_| anyhow!("サービスの発見に失敗しました。"))?;

        let characteristic = peripheral
            .characteristics()
            .into_iter()
            .find(|c| c.uuid == CHARACTERISTIC_UUID);

        let Some(characteristic) = characteristic else {
            warn!("指定されたキャラクタリスティックが見つかりません。再試行します...");
            time::sleep(Duration::from_secs(5)).await;
            continue;
        };

        for i in 0i32.. {
            let result = peripheral
                .write(&characteristic, &i.to_le_bytes(), WriteType::WithResponse)
                .await;
            if let Err(e) = result {
                warn!("送信時にエラーが発生しました: {:?}。", e);
            }

            info!("送信: {}", i);
            time::sleep(Duration::from_millis(rng.random_range(..=1500))).await;
        }
    }
}
