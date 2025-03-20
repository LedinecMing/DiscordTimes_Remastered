cd dt;
../build.sh quad_ui
../build.sh dt_server
cargo build --release -p quad_ui --target x86_64-pc-windows-gnu
../android_build.sh
yes | cp quad_ui dt/dt_r/DT_Remastered-LINUX
yes | cp ../target/x86_64-pc-windows-gnu/release/quad_ui.exe dt_r/DT_Remastered-WINDA
yes | cp ../target/android-artifacts/release/apk/quad_ui.apk ./DiscordTimesPVP.apk
yes | cp dt_server server_shit/.
zip DiscordTimesPvP.zip -r dt_r
tar czf server_shit.tar.gz server_shit
