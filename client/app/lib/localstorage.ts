export const setStorage = (key: string, value: any) => {
  if (typeof window !== "undefined" && window.localStorage) {
    if (value == null) {
      localStorage.removeItem(key);
    } else {
      localStorage.setItem(key, value);
    }
    return true;
  }
  return false;
};

export const getStorage = (key: string) => {
  if (typeof window !== "undefined" && window.localStorage) {
    return localStorage.getItem(key);
  }
  return null;
};

export const getArrayStorage = (key: string) => {
  if (typeof window !== "undefined" && window.localStorage) {
    const arr = localStorage.getItem(key);
    if (arr) {
      return JSON.parse(arr);
    }
    return null;
  }
};

export const addStorage = (key: string, value: any) => {
  if (typeof window !== "undefined" && window.localStorage) {
    if (value == null) {
      localStorage.removeItem(key);
    } else {
      const arr = localStorage.getItem(key);
      if (arr) {
        var parsed = JSON.parse(arr);
        parsed.push(value);
        localStorage.setItem(key, JSON.stringify(parsed));
      } else {
        return false;
      }
    }
    return true;
  }
  return false;
};

export class LocalStorage {
  static initStorage = () => {
    if (getStorage("isAuthenticated") == null) {
      setStorage("isAuthenticated", false);
    }
  };

  static setAuthenticated = (value: boolean) => {
    setStorage("isAuthenticated", value);
  };

  static isAuthenticated = (): boolean => {
    if (getStorage("isAuthenticated") === "true") return true;
    return false;
  };


  static setWallet = (value: string | null) => {
    setStorage("wallet", value);
  };

  static getWallet = () => {
    return getStorage("wallet");
  };

  static setUserAddress = (value: string | null) => {
    setStorage("userAddress", value);
  };

  static getUserAddress = () => {
    return getStorage("userAddress");
  };

  static setNetworkId = (value: string | null) => {
    setStorage("chainId", value);
  };

  static getNetworkId = () => {
    return getStorage("chainId");
  };


  static setICAAddress = ({
    networkId,
    userAddress,
    icaAddress,
  }: {
    networkId: string;
    userAddress: string;
    icaAddress: string;
  }) => {
    localStorage.setItem(`ica-${networkId}-${userAddress}`, icaAddress);
  };

  static getICAAddress = ({
    networkId,
    userAddress,
  }: {
    networkId: string;
    userAddress: string;
  }) => {
    return getStorage(`ica-${networkId}-${userAddress}`);
  };

}

