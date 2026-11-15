// Rufus Crush for Android: a thin shell around the web build. The whole game
// (HTML, JS loader, wasm) is packed into the APK as assets and shown in a
// WebView, so it needs no network at all. There are no library dependencies.
plugins {
    id("com.android.application")
}

android {
    namespace = "co.hoult.rufuscrush"
    compileSdk = 36

    defaultConfig {
        applicationId = "co.hoult.rufuscrush"
        minSdk = 30
        targetSdk = 36
        versionCode = 1
        versionName = "0.1.0"
    }

    buildTypes {
        release {
            // Signed with the debug key so the same machine can keep installing updates over it.
            signingConfig = signingConfigs.getByName("debug")
            isMinifyEnabled = false
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    androidResources {
        // Leave the wasm uncompressed in the APK so it can be streamed straight out of it.
        noCompress += "wasm"
    }
}
