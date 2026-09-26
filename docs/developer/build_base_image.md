# Build base-image

- Enable QEMU (for cross-architecture builds)

    ```bash
    docker run --privileged --rm tonistiigi/binfmt --install all
    ```

    This lets your x64 machine build ARM images locally

- Verify

    ```bash
    docker buildx ls
    ```

    ( You should see linux/amd64 and linux/arm64. )

- Create and use a Buildx builder

    ```bash
    docker buildx create --name multi-builder --use
    docker buildx inspect --bootstrap
    ```

- Login to Docker Hub

    ```bash
    docker login
    ```

- Build new base-image

    ```bash
    docker buildx build \
    --platform linux/amd64,linux/arm64 \
    -f dockerfiles/Dockerfile_build_base \
    -t kitsudaiki/ainari_build_base:0.1.0 \
    --push .
    ```
