package com.example.app

import io.flutter.embedding.android.FlutterActivity

class MainActivity: FlutterActivity() {

    companion object {
        init {
            // For things to work correctly native library must be loaded from Java first.
            System.loadLibrary("example_rust")
        }
    }
}
