cd ui; cargo build --release -p $1; cd ..; mv target/release/$1 dt/. -f && cd dt
