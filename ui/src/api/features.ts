import { loggedInvoke } from '@/utils/logged-invoke';

/** A feature flag definition. */
export interface FeatureInfo {
  key: string;
  name: string;
  description: string;
  group: string;
  enabled: boolean;
  dependencies: string[];
}

/** Response from listing all feature flags. */
export interface ListAllFeaturesResult {
  features: FeatureInfo[];
}

/** Response from toggling a single feature. */
export interface SetFeatureResult {
  success: boolean;
  features: FeatureInfo[];
  auto_enabled: string[];
}

/** List all feature flags with their current state. */
export const listAllFeatures = (): Promise<ListAllFeaturesResult> =>
  loggedInvoke<ListAllFeaturesResult>('list_all_features');

/** Toggle a single feature flag on or off. */
export const setFeature = (key: string, enabled: boolean): Promise<SetFeatureResult> =>
  loggedInvoke<SetFeatureResult>('set_feature', { args: { key, enabled } });

/** Bulk-toggle multiple feature flags. */
export const setFeaturesBulk = (keys: string[], enabled: boolean): Promise<ListAllFeaturesResult> =>
  loggedInvoke<ListAllFeaturesResult>('set_features_bulk', { args: { keys, enabled } });
