use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "resources/proto.pest"] // Path relative to the project root
pub struct ProtoParser;

// Test module.
#[cfg(test)]
mod tests {
    use super::*;
    use pest::iterators::Pairs;
    // Import ProtoParser and Rule from the parent module
    use pest::Parser;
    use std::fs;

    #[test]
    fn parses_order_proto_successfully() {
        let proto_content = fs::read_to_string("tests/resources/order.proto")
            .expect("Failed to read file 'tests/resources/order.proto'");
        let pairs =
            ProtoParser::parse(Rule::proto, &proto_content).expect("Parsing failed with an error");
        assert!(pairs.len() > 0, "Parser did not return any token pairs");
        let last_meaningful_pair = pairs.flatten().last().unwrap().as_rule();
        assert_eq!(last_meaningful_pair, Rule::EOI);
    }

    #[test]
    fn fails_on_invalid_syntax() {
        let invalid_proto = r#"
            syntax = "proto3";
            message Order {
                int32 id = 1
                string name = 2;
            }
        "#;
        let result = ProtoParser::parse(Rule::proto, invalid_proto);
        assert!(
            result.is_err(),
            "Parser should have returned an error, but it didn't"
        );
    }

    // --- NEW TEST AND HELPER ELEMENTS ---

    /// Helper struct to store extracted field information.
    /// `#[derive(Debug, PartialEq, Eq)]` allows us to print it to the console for debugging
    /// and easily compare instances in `assert_eq!`.
    #[derive(Debug, PartialEq, Eq)]
    struct FieldInfo {
        modifier: Option<String>,
        field_type: String,
        name: String,
        tag: u32,
    }

    /// Helper function to find a message by name and extract information about its fields.
    /// This makes the test itself much cleaner.
    fn extract_fields_from_message(root_pairs: &Pairs<Rule>, message_name: &str) -> Vec<FieldInfo> {
        let proto_pair = root_pairs.clone().next().expect("Expected a proto rule");

        let message_pair = proto_pair
            .into_inner()
            .flat_map(|pair| pair.into_inner())
            .flat_map(|pair| pair.into_inner())
            .find(|block_pair| {
                if block_pair.as_rule() == Rule::message_block {
                    return block_pair
                        .clone()
                        .into_inner()
                        .any(|p| p.as_rule() == Rule::message_name && p.as_str() == message_name);
                }
                false
            })
            .expect(&format!("Message '{}' not found", message_name));

        let message_body = message_pair
            .into_inner()
            .find(|pair| pair.as_rule() == Rule::message_body)
            .unwrap();

        // CORRECTED LOGIC HERE
        message_body
            .into_inner() // Get an iterator over message_elements
            .filter_map(|element_pair| {
                // Look inside the message_element. There should be one child: field, option, etc.
                let inner_pair = element_pair.into_inner().next().unwrap();

                // If this child is a field, process it.
                if inner_pair.as_rule() == Rule::field {
                    let mut inner_rules = inner_pair.into_inner();

                    let modifier = if inner_rules
                        .peek()
                        .map_or(false, |p| p.as_rule() == Rule::field_modifier)
                    {
                        Some(inner_rules.next().unwrap().as_str().to_string())
                    } else {
                        None
                    };

                    let field_type = inner_rules.next().unwrap().as_str().to_string();
                    let name = inner_rules.next().unwrap().as_str().to_string();
                    let tag = inner_rules.next().unwrap().as_str().parse::<u32>().unwrap();

                    // Return Some(..) so that filter_map adds this element to the final collection.
                    Some(FieldInfo {
                        modifier,
                        field_type,
                        name,
                        tag,
                    })
                } else {
                    // If it's not a field (e.g., an option_entry), return None to skip it.
                    None
                }
            })
            .collect()
    }

    #[test]
    fn parses_message_fields_correctly() {
        let proto_content = fs::read_to_string("tests/resources/order.proto")
            .expect("Failed to read file 'tests/resources/order.proto'");

        let pairs =
            ProtoParser::parse(Rule::proto, &proto_content).expect("Parsing failed with an error");

        // --- Check 'Order' message fields ---
        let order_fields = extract_fields_from_message(&pairs, "Order");
        let expected_order_fields = vec![
            FieldInfo {
                modifier: None,
                field_type: "int32".to_string(),
                name: "id".to_string(),
                tag: 1,
            },
            FieldInfo {
                modifier: None,
                field_type: "string".to_string(),
                name: "name".to_string(),
                tag: 2,
            },
            FieldInfo {
                modifier: Some("repeated".to_string()),
                field_type: "OrderItem".to_string(),
                name: "items".to_string(),
                tag: 3,
            },
        ];
        assert_eq!(
            order_fields, expected_order_fields,
            "Fields in 'Order' message were parsed incorrectly"
        );

        // --- Check 'OrderItem' message fields ---
        let order_item_fields = extract_fields_from_message(&pairs, "OrderItem");
        let expected_order_item_fields = vec![
            FieldInfo {
                modifier: None,
                field_type: "string".to_string(),
                name: "name".to_string(),
                tag: 1,
            },
            FieldInfo {
                modifier: Some("optional".to_string()),
                field_type: "int64".to_string(),
                name: "count".to_string(),
                tag: 2,
            },
            FieldInfo {
                modifier: None,
                field_type: "OrderItemType".to_string(),
                name: "type".to_string(),
                tag: 3,
            },
        ];
        assert_eq!(
            order_item_fields, expected_order_item_fields,
            "Fields in 'OrderItem' message were parsed incorrectly"
        );
    }
}
