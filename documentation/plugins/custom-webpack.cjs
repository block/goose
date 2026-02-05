module.exports = function () {
    return {
      name: 'custom-webpack-loaders',
      configureWebpack(config, isServer, utils) {
        // Add YAML loader
        config.module.rules.push({
          test: /\.ya?ml$/,
          use: 'yaml-loader',
        });
        
        // Add .raw file loader
        config.module.rules.push({
          test: /\.raw$/,
          type: 'asset/source',
        });
        
        return {};
      },
    };
  };

