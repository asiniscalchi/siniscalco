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
  fetchAssets,
  deleteAsset,
  extractGqlErrorMessage,
  type Asset,
} from "@/lib/api";

import { AssetFormModal } from "./AssetFormModal";
import { AssetsTableCard } from "./AssetsTableCard";

export function AssetsPage() {
  const [isLocked, setIsLocked] = useState(true);
  const [assets, setAssets] = useState<Asset[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [retryToken, setRetryToken] = useState(0);

  const [showModal, setShowModal] = useState(false);
  const [editingAsset, setEditingAsset] = useState<Asset | null>(null);
  const [isDeleting, setIsDeleting] = useState<number | null>(null);

  useEffect(() => {
    let cancelled = false;

    async function loadAssets() {
      setLoading(true);
      setError(null);

      try {
        const data = await fetchAssets();

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

  const handleEditClick = (asset: Asset) => {
    setEditingAsset(asset);
    setShowModal(true);
  };

  const handleDeleteClick = async (asset: Asset) => {
    if (!window.confirm(`Are you sure you want to delete ${asset.symbol}?`)) {
      return;
    }

    setIsDeleting(asset.id);
    try {
      await deleteAsset(asset.id);
      setRetryToken((t) => t + 1);
    } catch (error) {
      alert(extractGqlErrorMessage(error, "Failed to delete asset"));
    } finally {
      setIsDeleting(null);
    }
  };

  if (loading && assets.length === 0) {
    return (
      <div className="mx-auto flex w-full max-w-4xl flex-col gap-6">
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
      <AssetsTableCard
        assets={assets}
        isDeleting={isDeleting}
        isLocked={isLocked}
        onCreateClick={handleCreateClick}
        onDeleteClick={handleDeleteClick}
        onEditClick={handleEditClick}
        onToggleLock={() => setIsLocked((locked) => !locked)}
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
