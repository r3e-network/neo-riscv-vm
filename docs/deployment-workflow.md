# Deployment Workflow

## Step 1: Build Contract

```bash
./scripts/build-contract.sh
```

## Step 2: Generate Manifest

```bash
./scripts/generate-manifest.sh MyContract
```

## Step 3: Package

```bash
./scripts/package-contract.sh contract.polkavm manifest.json contract.nef
```

## Step 4: Deploy

```bash
./scripts/deploy-contract.sh contract.nef testnet
```
