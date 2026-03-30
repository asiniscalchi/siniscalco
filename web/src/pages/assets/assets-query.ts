import { gql } from "@apollo/client/core";

export const ASSETS_QUERY = gql`
  query Assets {
    assets {
      id symbol name assetType quoteSymbol isin
      currentPrice currentPriceCurrency currentPriceAsOf totalQuantity
      avgCostBasis avgCostBasisCurrency
      previousClose previousCloseCurrency
    }
  }
`;
