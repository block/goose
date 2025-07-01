module.exports = function () {
    return {
      name: 'custom-yaml-loader',
      configureWebpack() {
        return {
          module: {
            rules: [
              {
                test: /\.ya?ml$/,
                use: 'yaml-loader',
              },
            ],
          },
        };
      },
    };
  };
  