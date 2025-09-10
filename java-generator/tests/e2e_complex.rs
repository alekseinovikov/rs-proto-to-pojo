use std::collections::HashMap;
use std::path::Path;

use java_generator::generate_java_from_proto;

#[test]
fn e2e_generate_complex_proto() {
    let proto = Path::new("tests/resources/complex.proto");
    let files = generate_java_from_proto(proto).expect("generation should succeed");

    // Collect as map for easy assertions
    let map: HashMap<String, String> = files.into_iter().collect();

    // Expected relative paths include package as folder path
    let expected_paths = vec![
        "com/example/shop/Address.java",
        "com/example/shop/Customer.java",
        "com/example/shop/LineItem.java",
        "com/example/shop/Order.java",
        "com/example/shop/OrderStatus.java",
    ];

    for p in &expected_paths {
        assert!(map.contains_key(*p), "missing generated file: {}", p);
    }
    assert_eq!(
        map.len(),
        expected_paths.len(),
        "unexpected extra files generated"
    );

    // Check package line present in all
    for p in &expected_paths {
        let src = map.get(*p).unwrap();
        assert!(
            src.contains("package com.example.shop;"),
            "{} missing package",
            p
        );
    }

    // Address class content checks
    let addr = map.get("com/example/shop/Address.java").unwrap();
    assert!(addr.contains("public class Address"));
    assert!(addr.contains("private String street;"));
    assert!(addr.contains("private String city;"));
    assert!(addr.contains("private String state;"));
    assert!(addr.contains("private String zip;"));
    // getters/setters
    assert!(addr.contains("public String getStreet()"));
    assert!(addr.contains("public void setStreet(String value)"));

    // Enum content checks
    let status = map.get("com/example/shop/OrderStatus.java").unwrap();
    assert!(status.contains("public enum OrderStatus"));
    for v in [
        "UNKNOWN(0)",
        "PENDING(1)",
        "SHIPPED(2)",
        "DELIVERED(3)",
        "CANCELED(4)",
    ]
    .iter()
    {
        assert!(status.contains(v), "OrderStatus missing variant {v}");
    }
    assert!(status.contains("private final int number;"));
    assert!(status.contains("public int getNumber()"));

    // Order class with mixed types
    let order = map.get("com/example/shop/Order.java").unwrap();
    assert!(order.contains("public class Order"));
    assert!(order.contains("private String id;"));
    assert!(order.contains("private Customer customer;"));
    assert!(order.contains("private OrderStatus status;"));
    assert!(order.contains("private LineItem item;"));
    assert!(order.contains("private long created_at;"));

    // getters/setters for a few
    assert!(order.contains("public String getId()"));
    assert!(order.contains("public void setId(String value)"));
    assert!(order.contains("public Customer getCustomer()"));
    assert!(order.contains("public void setCustomer(Customer value)"));
    // Note: field name uses underscore; capitalize() keeps underscore, so method is getCreated_at
    assert!(order.contains("public long getCreated_at()"));
    assert!(order.contains("public void setCreated_at(long value)"));

    // Customer class referencing Address twice
    let customer = map.get("com/example/shop/Customer.java").unwrap();
    assert!(customer.contains("public class Customer"));
    assert!(customer.contains("private Address billing_address;"));
    assert!(customer.contains("private Address shipping_address;"));

    // LineItem with numeric scalars mapped correctly
    let item = map.get("com/example/shop/LineItem.java").unwrap();
    assert!(item.contains("private int quantity;"));
    assert!(item.contains("private double price;"));
}
