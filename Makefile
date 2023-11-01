all: build deploy

build:
	mkdir -p target-x86_64
	mkdir -p target-aarch64
	cargo build --release --target x86_64-unknown-linux-musl --target-dir target-x86_64
	cross build --release --target aarch64-unknown-linux-musl --target-dir target-aarch64
	docker buildx build --platform linux/amd64,linux/arm64/v8 . --tag registry.danya02.ru/unchessful/engine-directory:v1 --builder local --push

deploy:
	kubectl apply -f deploy.yaml

initialize_ns:
	kubectl create namespace buildkit

initialize_builder:
	docker buildx create --bootstrap --name=kube --driver=kubernetes --platform=linux/amd64 --node=builder-amd64 --driver-opt=namespace=buildkit,nodeselector="kubernetes.io/arch=amd64"
	docker buildx create --append --bootstrap --name=kube --driver=kubernetes --platform=linux/arm64 --node=builder-arm64 --driver-opt=namespace=buildkit,nodeselector="kubernetes.io/arch=arm64"

delete_builder:
	docker buildx rm kube