use serde::{Deserialize, Serialize};
use std::error::Error;
use std::io::Write;

#[derive(Debug, Serialize, Deserialize)]
struct OldEntry {
    id: String,
    task_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct NewEntry {
    id: String,
    task_id: String,
    #[serde(default)]
    task_title: String,
}

fn main() -> Result<(), Box<dyn Error>> {
    let path = "test_mixed.csv";

    {
        let mut wtr = csv::Writer::from_path(path)?;
        wtr.write_record(["id", "task_id"])?;
        wtr.serialize(OldEntry {
            id: "1".into(),
            task_id: "t1".into(),
        })?;
        wtr.flush()?;
    }

    {
        let mut file = std::fs::OpenOptions::new().append(true).open(path)?;
        writeln!(file, "2,t2,Title")?;
    }

    println!("Reading with NewEntry struct...");
    let mut rdr = csv::Reader::from_path(path)?;
    for result in rdr.deserialize::<NewEntry>() {
        match result {
            Ok(e) => println!("Row: {:?}", e),
            Err(e) => println!("Error: {}", e),
        }
    }

    std::fs::remove_file(path)?;
    Ok(())
}
