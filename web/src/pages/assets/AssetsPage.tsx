import { useState } from "react";
import { useMutation, useQuery } from "@apollo/client/react";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import {
  ASSETS_QUERY,
  DELETE_ASSET_MUTATION,
  extractGqlErrorMessage,
  type Asset,
} from "@/lib/api";

import { AssetFormModal } from "./AssetFormModal";
import { AssetsTableCard } from "./AssetsTableCard";

export function AssetsPage() {
  const [isLocked, setIsLocked] = useState(true);
  const [showModal, setShowModal] = useState(false);
  const [editingAsset, setEditingAsset] = useState<Asset | null>(null);
  const [isDeleting, setIsDeleting] = useState<number | null>(null);

  const { data, loading, error, refetch } = useQuery<{ assets: Asset[] }>(ASSETS_QUERY);
  const assets = data?.assets ?? [];

  const [deleteAssetMutation] = useMutation(DELETE_ASSET_MUTATION);

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
      await deleteAssetMutation({ variables: { id: asset.id } });
      await refetch();
    } catch (err) {
      alert(extractGqlErrorMessage(err, "Failed to delete asset"));
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
            <CardDescription>Failed to load assets</CardDescription>
          </CardHeader>
          <CardContent>
            <Button onClick={() => void refetch()}>Retry</Button>
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
        key={showModal ? (editingAsset?.id ?? "new") : "closed"}
        editingAsset={editingAsset}
        open={showModal}
        onClose={() => {
          setShowModal(false);
          setEditingAsset(null);
        }}
        onSaved={() => {
          setShowModal(false);
          setEditingAsset(null);
          void refetch();
        }}
      />
    </div>
  );
}
