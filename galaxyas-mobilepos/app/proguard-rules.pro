# kotlinx.serialization — pertahankan serializer yang dihasilkan compiler.
-keepattributes *Annotation*, InnerClasses
-dontnote kotlinx.serialization.**
-keepclassmembers class com.galaxyas.mobilepos.** {
    *** Companion;
}
-keepclasseswithmembers class com.galaxyas.mobilepos.** {
    kotlinx.serialization.KSerializer serializer(...);
}
-keep,includedescriptorclasses class com.galaxyas.mobilepos.**$$serializer { *; }
