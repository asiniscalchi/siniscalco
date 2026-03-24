import { useEffect, useState } from "react";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import {
  getAssetDetailApiUrl,
  getAssetsApiUrl,
  readApiErrorMessage,
  type AssetResponse,
} from "@/lib/api";

import { AssetFormModal } from "./AssetFormModal";
import { AssetPageHeader } from "./AssetPageHeader";
import { AssetsTableCard } from "./AssetsTableCard";

export function AssetsPage() {
  const [isLocked, setIsLocked] = useState(true);
  const [assets, setAssets] = useState<AssetResponse[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [retryToken, setRetryToken] = useState(0);

  const [showModal, setShowModal] = useState(false);
  const [editingAsset, setEditingAsset] = useState<AssetResponse | null>(null);
  const [isDeleting, setIsDeleting] = useState<number | null>(null);

  useEffect(() => {
    let cancelled = false;

    async function loadAssets() {
      setLoading(true);
      setError(null);

      try {
        const response = await fetch(getAssetsApiUrl());

        if (!response.ok) {
          throw new Error("Failed to load assets");
        }

        const data = (await response.json()) as AssetResponse[];

        if (!cancelled) {
          setAssets(data);
        }
      } catch {
        if (!cancelled) {
          setError("Failed to load assets");
        }
      } finally {
        if (!cancelled) {
          setLoading(false);
        }
      }
    }

    void loadAssets();

    return () => {
      cancelled = true;
    };
  }, [retryToken]);

  const handleCreateClick = () => {
    setEditingAsset(null);
    setShowModal(true);
  };

  const handleEditClick = (asset: AssetResponse) => {
    setEditingAsset(asset);
    setShowModal(true);
  };

  const handleDeleteClick = async (asset: AssetResponse) => {
    if (!window.confirm(`Are you sure you want to delete ${asset.symbol}?`)) {
      return;
    }

    setIsDeleting(asset.id);
    try {
      const response = await fetch(getAssetDetailApiUrl(asset.id), {
        method: "DELETE",
      });

      if (!response.ok) {
        const message = await readApiErrorMessage(
          response,
          "Failed to delete asset",
        );
        alert(message);
        return;
      }

      setRetryToken((t) => t + 1);
    } catch {
      alert("Network error while deleting asset");
    } finally {
      setIsDeleting(null);
    }
  };

  if (loading && assets.length === 0) {
    return (
      <div className="mx-auto flex w-full max-w-4xl flex-col gap-6">
        <header className="flex flex-col gap-4 rounded-xl border bg-background px-6 py-5 shadow-sm sm:flex-row sm:items-center sm:justify-between">
          <div className="space-y-1">
            <h1 className="text-2xl font-semibold tracking-tight">Assets</h1>
            <p className="max-w-2xl text-sm text-muted-foreground">
              Manage the assets you use in transactions.
            </p>
          </div>
          <div className="h-10 w-32 animate-pulse rounded-lg bg-muted" />
        </header>
        <div className="h-64 w-full animate-pulse rounded-xl bg-muted" />
      </div>
    );
  }

  if (error && assets.length === 0) {
    return (
      <div className="mx-auto flex w-full max-w-4xl flex-col gap-6 py-8">
        <Card className="border-destructive/30 bg-background">
          <CardHeader>
            <CardTitle>Error</CardTitle>
            <CardDescription>{error}</CardDescription>
          </CardHeader>
          <CardContent>
            <Button onClick={() => setRetryToken((t) => t + 1)}>Retry</Button>
          </CardContent>
        </Card>
      </div>
    );
  }

  return (
    <div className="mx-auto flex min-w-0 w-full max-w-4xl flex-col gap-6 overflow-x-hidden">
      <AssetPageHeader
        isLocked={isLocked}
        onCreateClick={handleCreateClick}
        onToggleLock={() => setIsLocked((locked) => !locked)}
      />

      <AssetsTableCard
        assets={assets}
        isDeleting={isDeleting}
        isLocked={isLocked}
        onCreateClick={handleCreateClick}
        onDeleteClick={handleDeleteClick}
        onEditClick={handleEditClick}
      />

      <AssetFormModal
        editingAsset={editingAsset}
        open={showModal}
        onClose={() => {
          setShowModal(false);
          setEditingAsset(null);
        }}
        onSaved={() => {
          setShowModal(false);
          setEditingAsset(null);
          setRetryToken((t) => t + 1);
        }}
      />
    </div>
  );
}
