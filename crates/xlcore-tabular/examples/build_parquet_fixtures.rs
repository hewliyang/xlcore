use std::sync::Arc;

use arrow_array::{
    builder::{Int32Builder, ListBuilder, MapBuilder, StringBuilder, StructBuilder},
    Array, BooleanArray, Date32Array, Float64Array, Int64Array, RecordBatch, StringArray,
    Time64MicrosecondArray, TimestampMillisecondArray,
};
use arrow_schema::{DataType, Field, Fields, Schema, TimeUnit};
use parquet::arrow::ArrowWriter;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let here =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/parquet");
    std::fs::create_dir_all(&here)?;

    write_primitives(&here.join("primitives.parquet"))?;
    write_temporal(&here.join("temporal.parquet"))?;
    write_nested(&here.join("nested.parquet"))?;
    println!("wrote 3 fixtures into {}", here.display());
    Ok(())
}

fn write_primitives(path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    let schema = Arc::new(Schema::new(vec![
        Field::new("name", DataType::Utf8, false),
        Field::new("age", DataType::Int64, false),
        Field::new("score", DataType::Float64, true),
        Field::new("active", DataType::Boolean, false),
    ]));
    let batch = RecordBatch::try_new(
        schema.clone(),
        vec![
            Arc::new(StringArray::from(vec!["Ada", "Grace", "Linus"])),
            Arc::new(Int64Array::from(vec![36, 85, 54])),
            Arc::new(Float64Array::from(vec![Some(0.95), None, Some(0.5)])),
            Arc::new(BooleanArray::from(vec![true, false, true])),
        ],
    )?;
    write(path, &schema, &batch)
}

fn write_temporal(path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    let schema = Arc::new(Schema::new(vec![
        Field::new("joined", DataType::Date32, false),
        Field::new(
            "last_login",
            DataType::Timestamp(TimeUnit::Millisecond, None),
            true,
        ),
        Field::new("time_of_day", DataType::Time64(TimeUnit::Microsecond), true),
    ]));
    let batch = RecordBatch::try_new(
        schema.clone(),
        vec![
            Arc::new(Date32Array::from(vec![19737, 7456, 16700])),
            Arc::new(TimestampMillisecondArray::from(vec![
                Some(1_700_000_000_000),
                None,
                Some(1_710_500_000_000),
            ])),
            Arc::new(Time64MicrosecondArray::from(vec![
                Some(9 * 3600 * 1_000_000),
                Some((14 * 3600 + 30 * 60 + 15) * 1_000_000),
                Some((23 * 3600 + 59 * 60 + 59) * 1_000_000),
            ])),
        ],
    )?;
    write(path, &schema, &batch)
}

fn write_nested(path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    let mut tags = ListBuilder::new(Int32Builder::new());
    for row in [&[1, 2, 3][..], &[][..], &[42][..]] {
        for v in row {
            tags.values().append_value(*v);
        }
        tags.append(true);
    }
    let tags_arr = tags.finish();

    let owner_fields: Fields = Fields::from(vec![
        Field::new("name", DataType::Utf8, false),
        Field::new("age", DataType::Int32, true),
    ]);
    let mut owner = StructBuilder::new(
        owner_fields.clone(),
        vec![
            Box::new(StringBuilder::new()),
            Box::new(Int32Builder::new()),
        ],
    );
    for (name, age) in [("Ada", Some(36)), ("Grace", None), ("Linus", Some(54))] {
        owner
            .field_builder::<StringBuilder>(0)
            .unwrap()
            .append_value(name);
        let age_b = owner.field_builder::<Int32Builder>(1).unwrap();
        match age {
            Some(v) => age_b.append_value(v),
            None => age_b.append_null(),
        }
        owner.append(true);
    }
    let owner_arr = owner.finish();

    let mut attrs = MapBuilder::new(None, StringBuilder::new(), Int32Builder::new());
    attrs.keys().append_value("k1");
    attrs.values().append_value(1);
    attrs.keys().append_value("k2");
    attrs.values().append_value(2);
    attrs.append(true)?;
    attrs.append(true)?;
    attrs.keys().append_value("only");
    attrs.values().append_value(99);
    attrs.append(true)?;
    let attrs_arr = attrs.finish();

    let schema = Arc::new(Schema::new(vec![
        Field::new(
            "tags",
            DataType::List(Arc::new(Field::new("item", DataType::Int32, true))),
            false,
        ),
        Field::new("owner", DataType::Struct(owner_fields), false),
        Field::new("attrs", attrs_arr.data_type().clone(), false),
    ]));
    let batch = RecordBatch::try_new(
        schema.clone(),
        vec![Arc::new(tags_arr), Arc::new(owner_arr), Arc::new(attrs_arr)],
    )?;
    write(path, &schema, &batch)
}

fn write(
    path: &std::path::Path,
    schema: &Arc<Schema>,
    batch: &RecordBatch,
) -> Result<(), Box<dyn std::error::Error>> {
    let file = std::fs::File::create(path)?;
    let mut writer = ArrowWriter::try_new(file, schema.clone(), None)?;
    writer.write(batch)?;
    writer.close()?;
    println!("wrote {}", path.display());
    Ok(())
}
