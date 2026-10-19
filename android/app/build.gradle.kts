plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
    id("org.jetbrains.kotlin.plugin.compose")
}

android {
    namespace = "me.leoank.rc2s2"
    compileSdk = 35

    defaultConfig {
        applicationId = "me.leoank.rc2s2"
        minSdk = 26
        targetSdk = 35
        versionCode = 1
        versionName = "0.1.0"
        ndk { abiFilters += "arm64-v8a" }
    }
    buildFeatures { compose = true }
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
    sourceSets["main"].java.srcDir(layout.buildDirectory.dir("generated/uniffi"))
}

kotlin { jvmToolchain(17) }

// The client core: cross-built for arm64 Android into jniLibs, with its Kotlin bindings
// generated from the built library (ADR-0009, ADR-0024). Runs in `nix develop .#android`.
val buildCore by tasks.registering(Exec::class) {
    workingDir = rootDir.parentFile
    commandLine("sh", "android/build-core.sh")
    inputs.dir(rootDir.parentFile.resolve("crates"))
    outputs.dir(layout.projectDirectory.dir("src/main/jniLibs"))
    outputs.dir(layout.buildDirectory.dir("generated/uniffi"))
}
tasks.named("preBuild") { dependsOn(buildCore) }

dependencies {
    val compose = platform("androidx.compose:compose-bom:2025.07.00")
    implementation(compose)
    implementation("androidx.compose.material3:material3")
    implementation("androidx.compose.ui:ui")
    implementation("androidx.compose.ui:ui-tooling-preview")
    implementation("androidx.activity:activity-compose:1.10.1")
    implementation("androidx.lifecycle:lifecycle-viewmodel-compose:2.9.2")
    implementation("org.jetbrains.kotlinx:kotlinx-coroutines-android:1.10.2")
    // The UniFFI Kotlin bindings load the core through JNA.
    implementation("net.java.dev.jna:jna:5.17.0@aar")
}
