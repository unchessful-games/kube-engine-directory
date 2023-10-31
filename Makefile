all: build deploy

build:
	docker buildx build --platform linux/amd64,linux/arm64/v8 . --tag registry.danya02.ru/unchessful/engine-directory:latest --builder local --push

deploy:
	kubectl apply -f deploy.yaml

initialize_ns:
	kubectl create namespace buildkit

initialize_builder:
	docker buildx create --bootstrap --name=kube --driver=kubernetes --platform=linux/amd64 --node=builder-amd64 --driver-opt=namespace=buildkit,nodeselector="kubernetes.io/arch=amd64"
	docker buildx create --append --bootstrap --name=kube --driver=kubernetes --platform=linux/arm64 --node=builder-arm64 --driver-opt=namespace=buildkit,nodeselector="kubernetes.io/arch=arm64"

delete_builder:
	docker buildx rm kube