use ::PbfParseError;
use blob::{Blob, BlobType};
use osm::{MemberReference, NANODEGREE_UNIT, NodeReference};
use protobuf;
use protos::osm::{DenseNodes, HeaderBlock, Node, PrimitiveBlock, PrimitiveGroup, Relation, StringTable, Way};
use std::io::Write;
use std::i64;
use std::ops;
use visitor::{BlobVisitor, OsmVisitor};

const MAX_ENTITY_COUNT: usize = 8000;

pub struct PrimitiveBlockBuilder {
    strings: Vec<String>,
    nodes: Vec<NodeEntity>,
    ways: Vec<WayEntity>,
    relations: Vec<RelationEntity>,
    entity_counter: u32,
    completed_blocks: Vec<PrimitiveBlock>,
}

impl PrimitiveBlockBuilder {
    fn new() -> PrimitiveBlockBuilder {
        PrimitiveBlockBuilder {
            strings: Vec::new(),
            nodes: Vec::new(),
            ways: Vec::new(),
            relations: Vec::new(),
            entity_counter: 0,
            completed_blocks: Vec::new(),
        }
    }

    fn append_node(&mut self, id: i64, latitude: f64, longitude: f64) {
        let lat_unit = (latitude / NANODEGREE_UNIT) as i64;
        let lon_unit = (longitude / NANODEGREE_UNIT) as i64;
        self.nodes.push(NodeEntity { id, latitude: lat_unit, longitude: lon_unit });
        self.complete_if_needed();
    }

    fn append_way(&mut self, id: i64, nodes: Vec<NodeReference>, tags: Vec<(String, String)>) {
        self.ways.push(WayEntity { id, nodes, tags });
        self.complete_if_needed();
    }

    fn append_relation(&mut self, id: i64, members: Vec<MemberReference>, tags: Vec<(String, String)>) {
        self.relations.push(RelationEntity { id, members, tags });
        self.complete_if_needed();
    }

    fn append_string(&mut self) {
        self.complete_if_needed();
    }

    #[inline]
    fn complete_if_needed(&mut self) {
        if self.get_entity_count() >= MAX_ENTITY_COUNT {
            self.complete_block();
        }
    }

    fn complete_block(&mut self) {
        let mut groups: Vec<PrimitiveGroup> = Vec::new();
        let pack_info = build_pack_info(&self.nodes);

        if !self.nodes.is_empty() {
            let mut node_group = PrimitiveGroup::default();
            let nodes = self.nodes.drain(ops::RangeFull).collect();
            node_group.set_dense(build_dense_nodes(nodes, &pack_info));
            groups.push(node_group);
        }

        if !self.ways.is_empty() {
            let mut way_group = PrimitiveGroup::default();
            let ways = self.ways.drain(ops::RangeFull).collect();
            way_group.set_ways(protobuf::RepeatedField::from_vec(build_ways(ways)));
            groups.push(way_group);
        }

        if !self.relations.is_empty() {
            let mut relation_group = PrimitiveGroup::default();
            let relations = self.relations.drain(ops::RangeFull).collect();
            relation_group.set_relations(protobuf::RepeatedField::from_vec(build_relations(relations)));
            groups.push(relation_group);
        }

        let mut block = PrimitiveBlock::default();
        block.set_primitivegroup(protobuf::RepeatedField::from_vec(groups));
        block.set_lat_offset(pack_info.lat_offset);
        block.set_lon_offset(pack_info.lon_offset);
        block.set_granularity(pack_info.granularity);
        block.set_date_granularity(1);

        let mut table = StringTable::default();
        table.set_s(protobuf::RepeatedField::new());
        block.set_stringtable(table);
        // TODO: Date granularity + string table

        self.completed_blocks.push(block);
    }

    #[inline]
    fn get_entity_count(&self) -> usize {
        self.nodes.len() + self.ways.len() + self.relations.len()
    }

    fn complete(&mut self) {
        if self.get_entity_count() > 0 {
            self.complete_block();
        }
    }

    fn take_blocks(&mut self) -> Vec<PrimitiveBlock> {
        let blocks = self.completed_blocks.to_owned();
        self.completed_blocks = Vec::new();
        blocks
    }
}

fn build_pack_info(nodes: &Vec<NodeEntity>) -> PackInfo {
    if !nodes.is_empty() {
        // TODO: Calculate granularity
        let mut lat_offset = i64::MAX;
        let mut lon_offset = i64::MAX;
        let granularity = 1;
        for node in nodes {
            if node.latitude < lat_offset {
                lat_offset = node.latitude;
            }
            if node.longitude < lon_offset {
                lon_offset = node.longitude;
            }
        }
        PackInfo { lat_offset, lon_offset, granularity }
    } else {
        PackInfo { lat_offset: 0, lon_offset: 0, granularity: 1 }
    }
}

fn build_dense_nodes(nodes: Vec<NodeEntity>, pack_info: &PackInfo) -> DenseNodes {
    let mut id = Vec::with_capacity(nodes.len());
    let mut lat = Vec::with_capacity(nodes.len());
    let mut lon = Vec::with_capacity(nodes.len());

    let mut prev_id = 0;
    let mut prev_lat = 0;
    let mut prev_lon = 0;

    for node in nodes {
        let local_id = node.id;
        let local_lat = node.latitude - pack_info.lat_offset;
        let local_lon = node.longitude - pack_info.lon_offset;

        id.push(local_id - prev_id);
        lat.push(local_lat - prev_lat);
        lon.push(local_lon - prev_lon);

        prev_id = local_id;
        prev_lat = local_lat;
        prev_lon = local_lon;
    }

    let mut dense_nodes = DenseNodes::default();
    dense_nodes.set_id(id);
    dense_nodes.set_lat(lat);
    dense_nodes.set_lon(lon);
    // TODO: Dense info
    dense_nodes
}

fn build_ways(ways: Vec<WayEntity>) -> Vec<Way> {
    ways.iter()
        .map(|way| {
            let mut out_way = Way::default();
            out_way.set_id(way.id);
            // TODO: refs, keys, vals
            out_way
        })
        .collect()
}

fn build_relations(relations: Vec<RelationEntity>) -> Vec<Relation> {
    relations.iter()
        .map(|relation| {
            let mut out_relation = Relation::default();
            out_relation.set_id(relation.id);
            // TODO: refs, keys, vals
            out_relation
        })
        .collect()
}

pub struct OsmWriterVisitor<'a> {
    writer: &'a mut Write,
    builder: PrimitiveBlockBuilder,
}

impl<'a> OsmWriterVisitor<'a> {
    pub fn new(writer: &'a mut Write) -> OsmWriterVisitor<'a> {
        OsmWriterVisitor {
            writer,
            builder: PrimitiveBlockBuilder::new(),
        }
    }

    fn write_completed(&mut self) -> Result<(), PbfParseError> {
        use protobuf::Message;;
        let completed = self.builder.take_blocks();
        for block in completed {
            let bytes = block.write_to_bytes()?;
            let blob = Blob::new(BlobType::DATA, bytes);
            blob.write(self.writer)?;
        }
        Ok(())
    }
}

impl<'a> OsmVisitor for OsmWriterVisitor<'a> {
    fn visit_block(&mut self, lat_offset: i64, lon_offset: i64, granularity: i32, date_granularity: i32) -> Result<(), PbfParseError> {
        Ok(())
    }

    fn visit_string_table(&mut self, strings: &Vec<&str>) -> Result<(), PbfParseError> {
        Ok(())
    }

    fn end_block(&mut self) -> Result<(), PbfParseError> {
        self.write_completed()?;
        Ok(())
    }

    fn visit_group(&mut self) -> Result<(), PbfParseError> {
        Ok(())
    }

    fn end_group(&mut self) -> Result<(), PbfParseError> {
        Ok(())
    }

    fn visit_node(&mut self, id: i64, latitude: f64, longitude: f64) -> Result<(), PbfParseError> {
        self.builder.append_node(id, latitude, longitude);
        Ok(())
    }

    fn visit_way(&mut self, id: i64, nodes: Vec<NodeReference>, tags: Vec<(String, String)>) -> Result<(), PbfParseError> {
        self.builder.append_way(id, nodes, tags);
        Ok(())
    }

    fn visit_relation(&mut self, id: i64, members: Vec<MemberReference>, tags: Vec<(String, String)>) -> Result<(), PbfParseError> {
        self.builder.append_relation(id, members, tags);
        Ok(())
    }

    fn visit_info(&mut self, version: i32, timestamp: i64, changeset: i64, uid: i32, user_sid: u32, visible: bool) -> Result<(), PbfParseError> {
        Ok(())
    }

    fn visit_header(&mut self, block: &HeaderBlock) -> Result<(), PbfParseError> {
        Ok(())
    }

    fn end(&mut self) -> Result<(), PbfParseError> {
        self.builder.complete();
        self.write_completed()?;
        Ok(())
    }
}

struct NodeEntity {
    id: i64,
    latitude: i64,
    longitude: i64,
}

struct WayEntity {
    id: i64,
    nodes: Vec<NodeReference>,
    tags: Vec<(String, String)>,
}

struct RelationEntity {
    id: i64,
    members: Vec<MemberReference>,
    tags: Vec<(String, String)>,
}

struct PackInfo {
    lat_offset: i64,
    lon_offset: i64,
    granularity: i32,
}

//pub struct PrimitiveBlockWriter<'a> {
//    writer: &'a mut Write,
//    block: Option<PrimitiveBlock>,
//    group: Option<PrimitiveGroup>,
//}
//
//impl<'a> PrimitiveBlockWriter<'a> {
//    fn new(writer: &'a mut Write) -> PrimitiveBlockWriter<'a> {
//        PrimitiveBlockWriter {
//            writer,
//            block: None,
//            group: None,
//        }
//    }
//
//    fn start_block(&mut self, lat_offset: i64, lon_offset: i64, granularity: i32, date_granularity: i32) {
//        let mut block = PrimitiveBlock::default();
//        block.set_lat_offset(lat_offset);
//        block.set_lon_offset(lon_offset);
//        block.set_granularity(granularity);
//        block.set_date_granularity(date_granularity);
//        self.block = Some(block);
//    }
//
//    fn end_block(&mut self) -> Result<(), PbfParseError> {
//        use protobuf::Message;
//        if let Some(block) = self.block.to_owned() {
//            let bytes = block.write_to_bytes()?;
//            let blob = Blob::new(BlobType::DATA, bytes);
//            blob.write(self.writer)?;
//            self.writer.flush()?;
//        }
//        self.block = None;
//        Ok(())
//    }
//
//    fn write_string_table(&mut self, strings: Vec<Vec<u8>>) -> Result<(), PbfParseError> {
//        if let Some(ref mut block) = self.block.as_mut() {
//            let mut table = StringTable::default();
//            table.set_s(protobuf::RepeatedField::from_vec(strings));
//            block.set_stringtable(table);
//        }
//        Ok(())
//    }
//
//    fn start_group(&mut self) {
//        self.group = Some(PrimitiveGroup::default());
//    }
//
//    fn end_group(&mut self) -> Result<(), PbfParseError> {
//        if let Some(group) = self.group.to_owned() {
//            if let Some(ref mut block) = self.block.as_mut() {
//                block.mut_primitivegroup().push(group);
//            }
//        }
//        self.group = None;
//        Ok(())
//    }
//}
