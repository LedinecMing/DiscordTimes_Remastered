export ANDROID_HOME=~/Android/sdk
export NDK_HOME=~/Android/android-ndk-r25b
export JAVA_HOME=/usr/lib/jvm/openjdk8
cd dt;
cp assets -r android_pack/.
cp *.ini android_pack/.
cp *.ttf android_pack/.
cp map -r android_pack/. -r
cargo quad-apk build --release -p quad_ui
