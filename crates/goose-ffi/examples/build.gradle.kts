plugins {
    kotlin("jvm") version "1.9.0"
    kotlin("plugin.serialization") version "1.9.0"
    application
}

repositories {
    mavenCentral()
}

dependencies {
    implementation("net.java.dev.jna:jna:5.13.0")
    implementation("org.jetbrains.kotlinx:kotlinx-serialization-json:1.6.0")
}

application {
    mainClass.set("com.example.goose.GooseExampleKt")
}

tasks.withType<org.jetbrains.kotlin.gradle.tasks.KotlinCompile> {
    kotlinOptions.jvmTarget = "11"
}

tasks.named<JavaExec>("run") {
    // Set the library path to find the Rust library
    systemProperty("jna.library.path", "../../../target/debug")
    
    // Pass through environment variables
    environment("DATABRICKS_API_KEY", System.getenv("DATABRICKS_API_KEY") ?: "")
    environment("DATABRICKS_HOST", System.getenv("DATABRICKS_HOST") ?: "")
}