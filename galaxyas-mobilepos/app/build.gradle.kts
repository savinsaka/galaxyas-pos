import java.util.Properties

plugins {
    alias(libs.plugins.android.application)
    alias(libs.plugins.kotlin.android)
    alias(libs.plugins.kotlin.compose)
    alias(libs.plugins.kotlin.serialization)
}

// Signing release dari keystore.properties (gitignored) — dibuat di Phase 5.
val keystoreProps = Properties().apply {
    val f = rootProject.file("keystore.properties")
    if (f.exists()) f.inputStream().use { load(it) }
}

android {
    namespace = "com.galaxyas.mobilepos"
    compileSdk = 36

    defaultConfig {
        applicationId = "com.galaxyas.mobilepos"
        minSdk = 24
        targetSdk = 36
        versionCode = 9
        versionName = "1.1.1"
    }

    signingConfigs {
        if (keystoreProps.isNotEmpty()) {
            create("release") {
                storeFile = file(keystoreProps.getProperty("storeFile"))
                storePassword = keystoreProps.getProperty("storePassword")
                keyAlias = keystoreProps.getProperty("keyAlias")
                keyPassword = keystoreProps.getProperty("keyPassword")
            }
        }
    }

    buildTypes {
        release {
            isMinifyEnabled = true
            isShrinkResources = true
            proguardFiles(getDefaultProguardFile("proguard-android-optimize.txt"), "proguard-rules.pro")
            if (keystoreProps.isNotEmpty()) signingConfig = signingConfigs.getByName("release")
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
    kotlinOptions {
        jvmTarget = "17"
    }
    buildFeatures {
        compose = true
        buildConfig = true
    }
}

/**
 * Sebar APK rilis otomatis sesudah `assembleRelease`, supaya tidak ada lagi
 * langkah salin manual yang bisa kelupaan:
 *   1. `galaxyas-mobilepos/dist/galaxyas-mobilepos-<versi>.apk` — arsip lokal per versi.
 *   2. Folder Google Drive (default `G:\My Drive\aplikasi pos`) — Drive for
 *      Desktop mengunggahnya sendiri, jadi APK langsung bisa diunduh dari HP.
 *
 * Folder Drive bisa dipindah lewat `local.properties` (gitignored):
 *     apkDriveDir=G:\\My Drive\\folder lain
 * Kalau foldernya tidak ada (Drive belum jalan / mesin lain), langkah Drive
 * DILEWATI dengan peringatan — build rilis tidak boleh gagal gara-gara ini.
 */
val localProps = Properties().apply {
    val f = rootProject.file("local.properties")
    if (f.exists()) f.inputStream().use { load(it) }
}

val sebarApkRilis by tasks.registering {
    description = "Salin APK rilis ke folder dist dan Google Drive."
    group = "distribution"
    // Selalu jalan: APK bisa saja sudah ada tapi folder tujuan sudah dibersihkan.
    outputs.upToDateWhen { false }
    doLast {
        val apk = layout.buildDirectory.file("outputs/apk/release/app-release.apk").get().asFile
        if (!apk.exists()) {
            logger.warn("APK rilis tidak ditemukan di ${apk.path} — penyebaran dilewati.")
            return@doLast
        }
        val namaFile = "galaxyas-mobilepos-${android.defaultConfig.versionName}.apk"

        val dist = rootProject.file("dist")
        dist.mkdirs()
        apk.copyTo(File(dist, namaFile), overwrite = true)
        logger.lifecycle("APK rilis -> ${File(dist, namaFile).path}")

        val driveDir = File(localProps.getProperty("apkDriveDir") ?: "G:\\My Drive\\aplikasi pos")
        if (driveDir.isDirectory) {
            apk.copyTo(File(driveDir, namaFile), overwrite = true)
            logger.lifecycle("APK rilis -> ${File(driveDir, namaFile).path} (Google Drive)")
        } else {
            logger.warn("Folder Drive '${driveDir.path}' tidak ada — salin ke Drive dilewati. " +
                "Atur 'apkDriveDir' di local.properties kalau lokasinya pindah.")
        }
    }
}

tasks.matching { it.name == "assembleRelease" }.configureEach {
    finalizedBy(sebarApkRilis)
}

dependencies {
    implementation(libs.androidx.core.ktx)
    implementation(libs.androidx.activity.compose)
    implementation(platform(libs.androidx.compose.bom))
    implementation(libs.androidx.compose.ui)
    implementation(libs.androidx.compose.ui.tooling.preview)
    implementation(libs.androidx.compose.material3)
    implementation(libs.androidx.compose.icons.extended)
    implementation(libs.androidx.navigation.compose)
    implementation(libs.androidx.lifecycle.viewmodel.compose)
    implementation(libs.androidx.lifecycle.runtime.compose)
    implementation(libs.kotlinx.serialization.json)
    implementation(libs.kotlinx.coroutines.android)
    implementation(libs.okhttp)
    implementation(libs.androidx.datastore.preferences)
    implementation(libs.camerax.core)
    implementation(libs.camerax.camera2)
    implementation(libs.camerax.lifecycle)
    implementation(libs.camerax.view)
    implementation(libs.mlkit.barcode)

    debugImplementation(libs.androidx.compose.ui.tooling)

    testImplementation(libs.junit)
    testImplementation(libs.okhttp.mockwebserver)
    testImplementation(libs.kotlinx.coroutines.test)
    testImplementation(libs.kotlinx.serialization.json)
}
