import '../src/rust/api/network.dart';

class NetworkResult {
  const NetworkResult(this.status, this.body);
  final int status;
  final String body;
}

abstract class TextFetcher {
  Future<NetworkResult> fetch(String url);
}

class NetworkRepository implements TextFetcher {
  @override
  Future<NetworkResult> fetch(String url) async {
    final result = await fetchHttpsText(url: url);
    return NetworkResult(result.status, result.body);
  }
}
