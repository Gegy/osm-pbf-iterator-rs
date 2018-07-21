use ::PbfParseError;
use blob::{Blob, BlobType};
use osm::{MemberReference, NANODEGREE_UNIT, NodeReference};
use protobuf;
use protos::osm::{DenseNodes, HeaderBlock, PrimitiveBlock, PrimitiveGroup, Relation, StringTable, Way};
use std::collections::HashMap;
use std::i64;
use std::io::Write;
use std::ops;
use visitor::OsmVisitor;

const MAX_ENTITY_COUNT: usize = 8000;

pub struct PrimitiveBlockBuilder {
    nodes: Vec<NodeEntity>,
    ways: Vec<WayEntity>,
    relations: Vec<RelationEntity>,
    completed_blocks: Vec<PrimitiveBlock>,
}

impl PrimitiveBlockBuilder {
    fn new() -> PrimitiveBlockBuilder {
        PrimitiveBlockBuilder {
            nodes: Vec::new(),
            ways: Vec::new(),
            relations: Vec::new(),
            completed_blocks: Vec::new(),
        }
    }

    fn append_node(&mut self, id: i64, latitude: f64, longitude: f64) {
        let lat_unit = (latitude / NANODEGREE_UNIT).floor() as i64;
        let lon_unit = (longitude / NANODEGREE_UNIT).floor() as i64;
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

    #[inline]
    fn complete_if_needed(&mut self) {
        if self.get_entity_count() >= MAX_ENTITY_COUNT {
            self.complete_block();
        }
    }

    fn complete_block(&mut self) {
        let mut groups: Vec<PrimitiveGroup> = Vec::new();
        let mut strings = ReverseStringTable::new();

        let ways: Vec<WayEntity> = self.ways.drain(ops::RangeFull).collect();
        let relations: Vec<RelationEntity> = self.relations.drain(ops::RangeFull).collect();

        let tags: Vec<(String, String)> = ways.iter().flat_map(|w| w.tags.clone())
            .chain(relations.iter().flat_map(|r| r.tags.clone()))
            .collect();

        for (k, v) in tags {
            strings.push_string(k.clone());
            strings.push_string(v.clone());
        }

        let pack_info = build_pack_info(&self.nodes);

        if !self.nodes.is_empty() {
            let mut node_group = PrimitiveGroup::default();
            let nodes = self.nodes.drain(ops::RangeFull).collect();
            node_group.set_dense(build_dense_nodes(nodes, &pack_info));
            groups.push(node_group);
        }

        if !self.ways.is_empty() {
            let mut way_group = PrimitiveGroup::default();
            way_group.set_ways(protobuf::RepeatedField::from_vec(build_ways(ways, &strings)));
            groups.push(way_group);
        }

        if !self.relations.is_empty() {
            let mut relation_group = PrimitiveGroup::default();
            relation_group.set_relations(protobuf::RepeatedField::from_vec(build_relations(relations, &strings)));
            groups.push(relation_group);
        }

        let mut block = PrimitiveBlock::default();
        block.set_primitivegroup(protobuf::RepeatedField::from_vec(groups));
        block.set_lat_offset(pack_info.lat_offset);
        block.set_lon_offset(pack_info.lon_offset);
        block.set_granularity(pack_info.granularity as i32);
        block.set_date_granularity(1);

        let mut table = StringTable::default();
        table.set_s(protobuf::RepeatedField::from_vec(strings.to_table()));
        block.set_stringtable(table);
        // TODO: Date granularity

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
        let mut lat_offset = i64::MAX;
        let mut lon_offset = i64::MAX;
        let granularity = 100;
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
        let local_lat = (node.latitude / pack_info.granularity) - pack_info.lat_offset;
        let local_lon = (node.longitude / pack_info.granularity) - pack_info.lon_offset;

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

fn build_ways(ways: Vec<WayEntity>, strings: &ReverseStringTable) -> Vec<Way> {
    ways.iter()
        .map(|way| {
            let mut out_way = Way::default();

            out_way.set_id(way.id);
            out_way.set_keys(way.tags.iter()
                .filter_map(|(k, _)| strings.lookup_string(k))
                .collect()
            );
            out_way.set_vals(way.tags.iter()
                .filter_map(|(_, v)| strings.lookup_string(v))
                .collect()
            );

            let mut prev_id = 0;
            let mut refs = Vec::new();
            for node in &way.nodes {
                let id = node.id;
                refs.push(id - prev_id);
                prev_id = id;
            }
            out_way.set_refs(refs);

            out_way
        })
        .collect()
}

fn build_relations(relations: Vec<RelationEntity>, strings: &ReverseStringTable) -> Vec<Relation> {
    relations.iter()
        .map(|relation| {
            let mut out_relation = Relation::default();

            out_relation.set_id(relation.id);
            out_relation.set_keys(relation.tags.iter()
                .filter_map(|(k, _)| strings.lookup_string(k))
                .collect()
            );
            out_relation.set_vals(relation.tags.iter()
                .filter_map(|(_, v)| strings.lookup_string(v))
                .collect()
            );

            let mut prev_id = 0;
            let mut member_ids = Vec::new();
            let mut roles = Vec::new();
            let mut types = Vec::new();

            for member in &relation.members {
                let id = member.id;

                member_ids.push(id - prev_id);
                roles.push(member.role_sid);
                types.push(member.entity_type.into());

                prev_id = id;
            }

            out_relation.set_memids(member_ids);
            out_relation.set_roles_sid(roles);
            out_relation.set_types(types);

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
        use protobuf::Message;
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
    fn end_block(&mut self) -> Result<(), PbfParseError> {
        self.write_completed()?;
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

    fn visit_header(&mut self, block: &HeaderBlock) -> Result<(), PbfParseError> {
        use protobuf::Message;

        let bytes = block.write_to_bytes()?;
        let blob = Blob::new(BlobType::HEADER, bytes);
        blob.write(self.writer)?;

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
    granularity: i64,
}

struct ReverseStringTable {
    strings: Vec<String>,
    reverse_strings: HashMap<String, usize>,
}

impl ReverseStringTable {
    fn new() -> ReverseStringTable {
        ReverseStringTable {
            strings: Vec::new(),
            reverse_strings: HashMap::new(),
        }
    }

    fn clear(&mut self) {
        self.strings.clear();
        self.reverse_strings.clear();
    }

    fn push_string(&mut self, string: String) {
        if !self.reverse_strings.contains_key(&string) {
            let last_index = self.strings.len();
            self.strings.push(string.clone());
            self.reverse_strings.insert(string, last_index);
        }
    }

    fn lookup_string(&self, string: &String) -> Option<u32> {
        self.reverse_strings.get(string.as_str()).map(|i| *i as u32)
    }

    fn to_table(&self) -> Vec<Vec<u8>> {
        self.strings.clone().into_iter()
            .map(|s| s.into_bytes())
            .collect()
    }
}
