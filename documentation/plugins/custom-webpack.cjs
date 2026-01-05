module.exports = function () {
    return {
      name: 'custom-webpack-loaders',
      configureWebpack(config) {
        config.module.rules.push({
          test: /\.ya?ml$/,
          use: 'yaml-loader',
        });
        config.module.rules.push({
          test: /\.raw$/,
          type: 'asset/source',
        });
        return {};
      },
    };
  };

