//! Lesing av JSON-filer som skal tåle gamle og ødelagte verdier.

use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;

/// Leser `json` oppå standardverdiene til `T`. Hvert felt prøves for seg: ukjente felt og
/// felt med feil type ignoreres, så et nytt felt bare må legges til i structen og i `Default`.
/// Er ikke `json` et objekt, gis standardverdiene.
pub fn over_default<T: Serialize + DeserializeOwned + Default>(json: &str) -> T {
    let Ok(Value::Object(saved)) = serde_json::from_str::<Value>(json) else {
        return T::default();
    };
    let Ok(Value::Object(mut merged)) = serde_json::to_value(T::default()) else {
        return T::default();
    };
    for (key, value) in saved {
        let Some(old) = merged.get(&key).cloned() else { continue };
        merged.insert(key.clone(), value);
        if serde_json::from_value::<T>(Value::Object(merged.clone())).is_err() {
            merged.insert(key, old);
        }
    }
    serde_json::from_value(Value::Object(merged)).unwrap_or_default()
}
