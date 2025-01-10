sudo apt-get install curl build-essential pkg-config libxcursor-dev
curl -OL 'https://www.x.org/pub/individual/data/xcursor-themes-1.0.7.tar.xz'
tar xf xcursor-themes-1.0.7.tar.gz
cd xcursor-themes-1.0.7.tar.gz
./configure
make