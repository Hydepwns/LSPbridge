plugins {
    kotlin("jvm") version "1.7.10"
    id("org.springframework.boot") version "2.7.0"
}

repositories {
    mavenCentral()
}

dependencies {
    // Production dependencies
    implementation("org.springframework.boot:spring-boot-starter-web")
    implementation("com.google.guava:guava:31.1-jre")
    implementation("org.jetbrains.kotlin:kotlin-stdlib")
    api("org.apache.commons:commons-lang3:3.12.0")
    
    // Test dependencies
    testImplementation("junit:junit:4.13.2")
    testImplementation("org.mockito:mockito-core:4.6.1")
    testImplementation("io.mockk:mockk:1.12.4")
    
    // Other configurations
    compileOnly("org.projectlombok:lombok:1.18.24")
}
EOF < /dev/null