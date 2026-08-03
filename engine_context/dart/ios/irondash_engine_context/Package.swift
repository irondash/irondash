// swift-tools-version: 5.9

import PackageDescription

let package = Package(
    name: "irondash_engine_context",
    platforms: [
        .iOS("15.0")
    ],
    products: [
        .library(name: "irondash-engine-context", targets: ["irondash_engine_context"])
    ],
    dependencies: [
        .package(name: "FlutterFramework", path: "../FlutterFramework")
    ],
    targets: [
        .target(
            name: "irondash_engine_context",
            dependencies: [
                .product(name: "FlutterFramework", package: "FlutterFramework")
            ],
            cSettings: [
                .headerSearchPath("include/irondash_engine_context")
            ]
        )
    ]
)
