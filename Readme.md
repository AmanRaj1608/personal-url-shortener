```sh
docker build -t url-shortener .

docker run -p 8000:8000 \
    -e MONGO_URI="" \
    url-shortener
```

```sh
curl -X POST -d "" https://s.amanraj.dev/shorten
```